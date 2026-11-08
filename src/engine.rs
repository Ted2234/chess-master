use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// How good a position is, from the point of view of the side to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    /// Advantage in centipawns: 100 is about one pawn. Negative means worse.
    Centipawns(i32),
    /// Forced mate in this many moves. Negative: the side to move gets mated.
    Mate(i32),
}

/// What the engine found in one position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub score: Score,
    /// The best move in UCI notation, like `e2e4`.
    pub best_move: String,
}

/// How long the engine may think.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchLimit {
    /// Think for this many milliseconds.
    Millis(u64),
}

impl SearchLimit {
    /// The UCI `go` command for this limit.
    fn go_command(self) -> String {
        match self {
            SearchLimit::Millis(ms) => format!("go movetime {ms}"),
        }
    }
}

/// Everything that can go wrong while talking to the engine.
#[derive(Debug)]
pub enum EngineError {
    /// The `stockfish` program couldn't be started (not installed?).
    NotFound(io::Error),
    /// Reading from or writing to the engine failed.
    Io(io::Error),
    /// The engine stopped answering (it closed its output).
    Stopped,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotFound(error) => {
                write!(f, "could not start Stockfish ({error}). Is it installed?")
            }
            EngineError::Io(error) => write!(f, "talking to Stockfish failed: {error}"),
            EngineError::Stopped => write!(f, "Stockfish stopped unexpectedly"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Lets `?` turn an `io::Error` into an `EngineError` automatically.
impl From<io::Error> for EngineError {
    fn from(error: io::Error) -> Self {
        EngineError::Io(error)
    }
}

/// A running Stockfish process that we talk to through its stdin and stdout.
pub struct Engine {
    process: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Engine {
    /// Starts Stockfish and does the UCI handshake.
    pub fn start() -> Result<Self, EngineError> {
        let mut process = Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(EngineError::NotFound)?;
        let input = process.stdin.take().ok_or(EngineError::Stopped)?;
        let output = BufReader::new(process.stdout.take().ok_or(EngineError::Stopped)?);

        let mut engine = Engine {
            process,
            input,
            output,
        };
        engine.send("uci")?;
        engine.wait_for("uciok")?;
        engine.wait_until_ready()?;
        Ok(engine)
    }

    /// Tells the engine a new game starts, so it forgets the old one.
    pub fn new_game(&mut self) -> Result<(), EngineError> {
        self.send("ucinewgame")?;
        self.wait_until_ready()
    }

    /// Thinks about the position (given as FEN) and returns the best move and the score.
    pub fn search(&mut self, fen: &str, limit: SearchLimit) -> Result<Analysis, EngineError> {
        self.send(&format!("position fen {fen}"))?;
        self.send(&limit.go_command())?;

        let mut score = Score::Centipawns(0);
        loop {
            let line = self.read_line()?;
            // "info ..." lines come while it thinks; the last score is the final one.
            if let Some(found) = parse_score(&line) {
                score = found;
            }
            if let Some(best_move) = parse_best_move(&line) {
                return Ok(Analysis { score, best_move });
            }
        }
    }

    /// Sends one command line to the engine.
    fn send(&mut self, command: &str) -> Result<(), EngineError> {
        writeln!(self.input, "{command}")?;
        self.input.flush()?;
        Ok(())
    }

    /// Reads one line of engine output (blocks until there is one).
    fn read_line(&mut self) -> Result<String, EngineError> {
        let mut line = String::new();
        if self.output.read_line(&mut line)? == 0 {
            return Err(EngineError::Stopped); // 0 bytes: the engine closed its output
        }
        Ok(line.trim_end().to_string())
    }

    /// Reads lines until one equals `expected`.
    fn wait_for(&mut self, expected: &str) -> Result<(), EngineError> {
        while self.read_line()? != expected {}
        Ok(())
    }

    /// Asks "isready" and waits for "readyok".
    fn wait_until_ready(&mut self) -> Result<(), EngineError> {
        self.send("isready")?;
        self.wait_for("readyok")
    }
}

impl Drop for Engine {
    /// Asks Stockfish to quit when the `Engine` goes away, so no process is left behind.
    fn drop(&mut self) {
        // Errors are ignored on purpose: we're shutting down anyway.
        let _ = self.send("quit");
        let _ = self.process.wait();
    }
}

/// Reads the score from a line like `info depth 12 ... score cp 34 ...`.
fn parse_score(line: &str) -> Option<Score> {
    if !line.starts_with("info") {
        return None;
    }
    let mut words = line.split_whitespace();
    words.find(|&word| word == "score")?;
    let kind = words.next()?;
    let value: i32 = words.next()?.parse().ok()?;
    match kind {
        "cp" => Some(Score::Centipawns(value)),
        "mate" => Some(Score::Mate(value)),
        _ => None,
    }
}

/// Reads the move from a line like `bestmove e2e4 ponder e7e5`.
fn parse_best_move(line: &str) -> Option<String> {
    let rest = line.strip_prefix("bestmove ")?;
    rest.split_whitespace().next().map(str::to_string)
}

/// A job for the engine thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRequest {
    NewGame,
    /// Find the best move. `id` comes back with the answer so you know which request it was.
    Search {
        id: u64,
        fen: String,
        limit: SearchLimit,
    },
}

/// An answer from the engine thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineReply {
    Searched { id: u64, analysis: Analysis },
    Failed(String),
}

/// The GUI's end of the engine thread: send jobs, collect answers, never wait.
pub struct EngineHandle {
    requests: Sender<EngineRequest>,
    replies: Receiver<EngineReply>,
}

impl EngineHandle {
    /// Starts Stockfish and a thread that runs it.
    pub fn spawn() -> Result<Self, EngineError> {
        // Start the engine here, so an error (e.g. not installed) is reported right away.
        let mut engine = Engine::start()?;

        let (request_sender, request_receiver) = mpsc::channel::<EngineRequest>();
        let (reply_sender, reply_receiver) = mpsc::channel::<EngineReply>();

        // `move` gives the thread ownership of `engine` and the channel ends it uses.
        thread::spawn(move || {
            // The loop ends when the EngineHandle (the sender) is dropped.
            for request in request_receiver {
                if let Some(reply) = run_request(&mut engine, request)
                    && reply_sender.send(reply).is_err()
                {
                    break; // nobody is listening anymore
                }
            }
        });

        Ok(EngineHandle {
            requests: request_sender,
            replies: reply_receiver,
        })
    }

    /// Hands a job to the engine thread. Returns immediately.
    pub fn send(&self, request: EngineRequest) {
        // If the thread has died, the error shows up as a missing reply instead.
        let _ = self.requests.send(request);
    }

    /// Returns an answer if one has arrived, without waiting.
    pub fn try_receive(&self) -> Option<EngineReply> {
        self.replies.try_recv().ok()
    }
}

/// Runs one job on the engine thread. Only searches (and failures) send a reply.
fn run_request(engine: &mut Engine, request: EngineRequest) -> Option<EngineReply> {
    let outcome = match request {
        EngineRequest::NewGame => engine.new_game().map(|()| None),
        EngineRequest::Search { id, fen, limit } => engine
            .search(&fen, limit)
            .map(|analysis| Some(EngineReply::Searched { id, analysis })),
    };
    outcome.unwrap_or_else(|error| Some(EngineReply::Failed(error.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    #[test]
    fn parses_scores() {
        let line = "info depth 18 seldepth 25 multipv 1 score cp 34 nodes 512 pv e2e4 e7e5";
        assert_eq!(parse_score(line), Some(Score::Centipawns(34)));
        let line = "info depth 30 score mate -3 nodes 1000 pv h7h8";
        assert_eq!(parse_score(line), Some(Score::Mate(-3)));
        assert_eq!(parse_score("info string NNUE enabled"), None);
        assert_eq!(parse_score("bestmove e2e4"), None);
    }

    #[test]
    fn parses_best_move() {
        assert_eq!(
            parse_best_move("bestmove e2e4 ponder e7e5"),
            Some("e2e4".to_string())
        );
        assert_eq!(parse_best_move("bestmove e7e8q"), Some("e7e8q".to_string()));
        assert_eq!(parse_best_move("info depth 1"), None);
    }

    #[test]
    fn go_commands() {
        assert_eq!(SearchLimit::Millis(500).go_command(), "go movetime 500");
    }

    /// Needs Stockfish installed. Run with: cargo test -- --ignored
    #[test]
    #[ignore]
    fn real_engine_finds_a_move() {
        let mut engine = Engine::start().unwrap();
        let analysis = engine.search(START_FEN, SearchLimit::Millis(200)).unwrap();
        assert_eq!(analysis.best_move.len(), 4);

        // Mate in one: Ra8#.
        let analysis = engine
            .search(
                "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1",
                SearchLimit::Millis(200),
            )
            .unwrap();
        assert_eq!(analysis.best_move, "a1a8");
        assert_eq!(analysis.score, Score::Mate(1));
    }
}
