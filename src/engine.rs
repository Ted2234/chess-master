use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

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
    /// Look this many moves deep.
    Depth(u32),
}

impl SearchLimit {
    /// The UCI `go` command for this limit.
    fn go_command(self) -> String {
        match self {
            SearchLimit::Millis(ms) => format!("go movetime {ms}"),
            SearchLimit::Depth(depth) => format!("go depth {depth}"),
        }
    }
}

/// The file name of the Stockfish program on this platform. On Android it travels with
/// the app's libraries, and Android only unpacks files named `lib*.so` there.
const STOCKFISH_FILE: &str = if cfg!(windows) {
    "stockfish.exe"
} else if cfg!(target_os = "android") {
    "libstockfish.so"
} else {
    "stockfish"
};

/// Returns where to start Stockfish from: the copy bundled next to the app if there is one,
/// otherwise just `stockfish`, which the system looks up on the `PATH`.
pub fn stockfish_path() -> PathBuf {
    app_folder()
        .and_then(|folder| bundled_stockfish(&folder))
        .unwrap_or_else(|| PathBuf::from(STOCKFISH_FILE))
}

/// Returns the folder the app's program is in.
#[cfg(not(target_os = "android"))]
fn app_folder() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(Path::to_path_buf)
}

/// On Android the app is a library, `libchess_master.so`, loaded into a Java process, so
/// `current_exe` would name that process. `/proc/self/maps` lists every file this process
/// has loaded; our library's folder is where Android unpacked Stockfish too.
#[cfg(target_os = "android")]
fn app_folder() -> Option<PathBuf> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    library_folder(&maps, "libchess_master.so")
}

/// Finds the folder of the file called `name` in the text of `/proc/self/maps`.
/// Each line is `address perms offset device inode path`; the path may be missing.
#[cfg(any(target_os = "android", test))]
fn library_folder(maps: &str, name: &str) -> Option<PathBuf> {
    maps.lines()
        .filter_map(|line| line.split_whitespace().nth(5))
        .map(Path::new)
        .find(|path| path.file_name().is_some_and(|file| file == name))
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// Returns the Stockfish program inside `dir`, if it's there.
fn bundled_stockfish(dir: &Path) -> Option<PathBuf> {
    let path = dir.join(STOCKFISH_FILE);
    path.is_file().then_some(path)
}

/// On Windows, stops Stockfish from opening its own black console window.
#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

/// Other systems don't open a window for a background program, so there's nothing to do.
#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

/// Everything that can go wrong while talking to the engine.
#[derive(Debug)]
pub enum EngineError {
    /// The Stockfish program at `path` couldn't be started.
    NotFound { path: PathBuf, error: io::Error },
    /// Reading from or writing to the engine failed.
    Io(io::Error),
    /// The engine stopped answering (it closed its output).
    Stopped,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotFound { path, error } => write!(
                f,
                "could not start Stockfish from {} ({error}). It should be next to the app.",
                path.display()
            ),
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
        let path = stockfish_path();
        let mut command = Command::new(&path);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());
        hide_console_window(&mut command);
        let mut process = command
            .spawn()
            .map_err(|error| EngineError::NotFound { path, error })?;
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

    /// Changes an engine settings e.g. `set_option("Skill level", "5")`.
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<(), EngineError> {
        self.send(&format!("setoption name {name} value {value}"))?;
        self.wait_until_ready()
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
    /// Engine settings as (name, value) pairs.
    SetOptions(Vec<(String, String)>),
    /// Find the best move. `id` comes back with the answer so you know which request it was.
    Search {
        id: u64,
        fen: String,
        limit: SearchLimit,
        /// Answer no sooner than this after the request (`Duration::ZERO`: as soon as possible).
        min_time: Duration,
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
        EngineRequest::SetOptions(options) => options
            .iter()
            .try_for_each(|(name, value)| engine.set_option(name, value))
            .map(|()| None),
        EngineRequest::Search {
            id,
            fen,
            limit,
            min_time,
        } => {
            let started = Instant::now();
            let result = engine.search(&fen, limit);
            // Only a successful search waits; an error is reported right away.
            if result.is_ok() {
                thread::sleep(min_time.saturating_sub(started.elapsed()));
            }
            result.map(|analysis| Some(EngineReply::Searched { id, analysis }))
        }
    };
    outcome.unwrap_or_else(|error| Some(EngineReply::Failed(error.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::START_FEN;

    #[test]
    fn finds_bundled_stockfish() {
        let dir = std::env::temp_dir().join("chess-master-test-bundled");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(STOCKFISH_FILE);

        let _ = std::fs::remove_file(&path);
        assert_eq!(bundled_stockfish(&dir), None);

        std::fs::write(&path, "").unwrap();
        assert_eq!(bundled_stockfish(&dir), Some(path.clone()));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn finds_library_folder_in_maps() {
        let maps = "\
7f0000-7f1000 r--p 00000000 fd:00 123  /system/lib64/libc.so
7f2000-7f3000 rw-p 00000000 00:00 0
7f4000-7f5000 r-xp 00001000 fd:05 456  /data/app/~~a1==/chess-AB==/lib/arm64/libchess_master.so
";
        assert_eq!(
            library_folder(maps, "libchess_master.so"),
            Some(PathBuf::from("/data/app/~~a1==/chess-AB==/lib/arm64"))
        );
        assert_eq!(library_folder(maps, "libmissing.so"), None);
    }

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
        assert_eq!(SearchLimit::Depth(12).go_command(), "go depth 12");
    }

    /// Needs Stockfish installed. Run with: cargo test -- --ignored
    #[test]
    #[ignore]
    fn real_engine_finds_a_move() {
        let mut engine = Engine::start().unwrap();
        let analysis = engine.search(START_FEN, SearchLimit::Depth(8)).unwrap();
        assert_eq!(analysis.best_move.len(), 4);

        // Mate in one: Ra8#.
        let analysis = engine
            .search("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", SearchLimit::Depth(5))
            .unwrap();
        assert_eq!(analysis.best_move, "a1a8");
        assert_eq!(analysis.score, Score::Mate(1));
    }
}
