use crate::analysis;
use crate::board::Board;
use crate::settings::BoardTheme;
use crate::square::Square;
use crate::ui::piece_textures::PieceTextures;
use crate::ui::theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Shadow, Stroke, Vec2,
};

/// How round the four corners of the board are.
const BOARD_ROUNDING: u8 = 6;
/// The piece images are 32×32 pixels.
const ART_SIZE: f32 = 32.0;
/// Which part of a piece image to draw: (0,0) is its top-left corner, (1,1) its bottom-right.
const WHOLE_IMAGE: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
/// A square may shrink by up to this share (15%) to keep the pixel art sharp.
const MAX_SHRINK: f32 = 0.15;
const LAST_MOVE: Color32 = Color32::from_rgba_unmultiplied_const(255, 230, 50, 80);
const SELECTED: Color32 = Color32::from_rgba_unmultiplied_const(255, 230, 50, 140);
const MOVE_DOT: Color32 = Color32::from_rgba_unmultiplied_const(0, 0, 0, 45);
/// The width of the evaluation bar and the gap between it and the board.
const EVAL_BAR_WIDTH: f32 = 26.0;
const EVAL_BAR_GAP: f32 = 10.0;
/// One layer of the check glow. Several layers on top of each other make the middle redder.
const CHECK_GLOW: Color32 = Color32::from_rgba_unmultiplied_const(230, 30, 30, 40);
/// The two colors of the evaluation bar.
const EVAL_WHITE: Color32 = Color32::from_rgb(240, 240, 235);
const EVAL_BLACK: Color32 = Color32::from_rgb(40, 40, 40);
/// Arrow colors: a hint, the move that was played, and the engine's better move.
pub const HINT_ARROW: Color32 = Color32::from_rgba_unmultiplied_const(0, 120, 255, 170);
pub const PLAYED_MOVE_ARROW: Color32 = Color32::from_rgba_unmultiplied_const(230, 60, 60, 150);
pub const BEST_MOVE_ARROW: Color32 = Color32::from_rgba_unmultiplied_const(40, 180, 60, 180);

/// What the player did with the mouse on the board this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardInput {
    /// Pressed and let go on a square, without moving the mouse.
    Click(Square),
    /// Pressed on a square and started to drag.
    DragStart(Square),
    /// Let go of a drag above a square.
    Drop(Square),
}

/// A piece on its way from one square to another.
#[derive(Debug, Clone, Copy)]
pub struct Slide {
    pub from: Square,
    pub to: Square,
    /// How far it has come: 0.0 at `from`, 1.0 at `to`.
    pub progress: f32,
}

/// Everything that is drawn on top of the plain board.
#[derive(Debug, Clone, Default)]
pub struct Highlights {
    /// The squares the last move came from and went to.
    pub last_move: Option<(Square, Square)>,
    /// The square of the piece the player picked up.
    pub selected: Option<Square>,
    /// Where the selected piece can move to (dots).
    pub targets: Vec<Square>,
    /// The square of a king that is in check (drawn red).
    pub check: Option<Square>,
    /// Arrows from one square to another, each with its own color.
    pub arrows: Vec<(Square, Square, Color32)>,
    /// The piece of the last move, while it's still sliding to its square.
    pub slide: Option<Slide>,
    /// The evaluation for White in centipawns, shown as a bar next to the board.
    pub eval: Option<i32>,
}

/// Where the board is on the screen and how big each square is.
struct Layout {
    origin: Pos2,
    square_size: f32,
    flipped: bool,
}

impl Layout {
    /// Places the biggest board that fits in `area`, centered in it.
    fn new(area: Rect, pixels_per_point: f32, flipped: bool) -> Self {
        let square_size = square_size(area.size(), pixels_per_point);
        let top_left = area.center() - Vec2::splat(square_size * 4.0);
        Self {
            // Start on a whole screen pixel, so square edges are sharp too.
            origin: (top_left * pixels_per_point).round() / pixels_per_point,
            square_size,
            flipped,
        }
    }

    /// Returns the screen rectangle of the whole board.
    fn board_rect(&self) -> Rect {
        Rect::from_min_size(self.origin, Vec2::splat(self.square_size * 8.0))
    }

    /// Converts a screen column and row to a board square.
    fn square_from_screen(&self, col: usize, row: usize) -> Square {
        if self.flipped {
            Square::new(row, 7 - col)
        } else {
            Square::new(7 - row, col)
        }
    }

    /// Returns the screen column and row of a square; (0, 0) is the top-left corner.
    fn screen_cell(&self, square: Square) -> (usize, usize) {
        if self.flipped {
            (7 - square.file, square.rank)
        } else {
            (square.file, 7 - square.rank)
        }
    }

    /// Returns the screen rectangle of a square.
    fn square_rect(&self, square: Square) -> Rect {
        let (col, row) = self.screen_cell(square);
        let min = self.origin + Vec2::new(col as f32, row as f32) * self.square_size;
        Rect::from_min_size(min, Vec2::splat(self.square_size))
    }

    /// Returns how round each corner of a square is: only the corners of the board are.
    fn corner_radius(&self, square: Square) -> CornerRadius {
        let (col, row) = self.screen_cell(square);
        let round = |is_corner: bool| if is_corner { BOARD_ROUNDING } else { 0 };
        CornerRadius {
            nw: round(col == 0 && row == 0),
            ne: round(col == 7 && row == 0),
            sw: round(col == 0 && row == 7),
            se: round(col == 7 && row == 7),
        }
    }

    /// Returns the square under a screen position, or `None` if it's outside the board.
    fn square_at(&self, pos: Pos2) -> Option<Square> {
        let x = (pos.x - self.origin.x) / self.square_size;
        let y = (pos.y - self.origin.y) / self.square_size;
        if !(0.0..8.0).contains(&x) || !(0.0..8.0).contains(&y) {
            return None;
        }
        Some(self.square_from_screen(x as usize, y as usize))
    }
}

/// Returns the size of one square, in points, for a board that fits in `available`.
///
/// A square of 64 or 96 screen pixels shows every pixel of the 32×32 art as a 2×2 or 3×3
/// block, so the art stays sharp. If the next smaller multiple of 32 isn't much smaller
/// than what fits, use it; otherwise use all the room, in whole pixels.
fn square_size(available: Vec2, pixels_per_point: f32) -> f32 {
    let fit = (available.min_elem() / 8.0 * pixels_per_point).floor(); // in screen pixels
    let sharp = (fit / ART_SIZE).floor() * ART_SIZE;
    let pixels = if sharp >= fit * (1.0 - MAX_SHRINK) {
        sharp
    } else {
        fit
    };
    pixels / pixels_per_point
}

/// Draws the board and returns what the player did on it this frame, if anything.
pub fn draw_board(
    ui: &mut egui::Ui,
    board: &Board,
    textures: &PieceTextures,
    highlights: &Highlights,
    flipped: bool,
    board_theme: BoardTheme,
) -> Option<BoardInput> {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let mut board_area = response.rect;
    if highlights.eval.is_some() {
        // Room for the bar on the left; the same on the right keeps the board centered.
        board_area = board_area.shrink2(Vec2::new(EVAL_BAR_WIDTH + EVAL_BAR_GAP, 0.0));
    }
    let layout = Layout::new(board_area, ui.ctx().pixels_per_point(), flipped);
    let square_under = |pos: Option<Pos2>| pos.and_then(|pos| layout.square_at(pos));

    // While the selected piece is being dragged, it follows the mouse.
    let press_square = square_under(ui.input(|input| input.pointer.press_origin()));
    let dragged = match (highlights.selected, response.interact_pointer_pos()) {
        (Some(square), Some(pointer)) if response.dragged() && press_square == Some(square) => {
            Some((square, pointer))
        }
        _ => None,
    };

    // Later shapes cover earlier ones, so the order matters.
    draw_shadow(&painter, &layout);
    if let Some(white_cp) = highlights.eval {
        draw_eval_bar(ui.ctx(), &painter, &layout, white_cp);
    }
    draw_squares(&painter, &layout, board_theme);
    if let Some((from, to)) = highlights.last_move {
        tint_square(&painter, &layout, from, LAST_MOVE);
        tint_square(&painter, &layout, to, LAST_MOVE);
    }
    if let Some(square) = highlights.selected {
        tint_square(&painter, &layout, square, SELECTED);
    }
    if let Some(square) = highlights.check {
        draw_check_glow(&painter, &layout, square);
    }
    draw_label(&painter, &layout, board_theme);
    let hidden = dragged.map(|(square, _)| square);
    draw_pieces(&painter, &layout, board, textures, highlights.slide, hidden);
    draw_move_dots(&painter, &layout, board, &highlights.targets);
    for &(from, to, color) in &highlights.arrows {
        draw_arrow(&painter, &layout, from, to, color);
    }
    if let Some((square, pointer)) = dragged
        && let Some(piece) = board.get(square)
    {
        // On top of everything else, centered on the mouse.
        let rect = Rect::from_center_size(pointer, Vec2::splat(layout.square_size));
        painter.image(textures.get(piece), rect, WHOLE_IMAGE, Color32::WHITE);
    }

    let pointer_square = square_under(response.interact_pointer_pos());
    if response.drag_started() {
        press_square.map(BoardInput::DragStart)
    } else if response.drag_stopped() {
        pointer_square.map(BoardInput::Drop)
    } else if response.clicked() {
        pointer_square.map(BoardInput::Click)
    } else {
        None
    }
}

/// Paints a soft shadow under the board, a little lower than the board itself.
fn draw_shadow(painter: &Painter, layout: &Layout) {
    let shadow = Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    painter.add(shadow.as_shape(layout.board_rect(), BOARD_ROUNDING));
}

/// Paints the evaluation bar left of the board. White's part grows when White is better.
fn draw_eval_bar(ctx: &egui::Context, painter: &Painter, layout: &Layout, white_cp: i32) {
    let board = layout.board_rect();
    let right = board.left() - EVAL_BAR_GAP;
    let bar = Rect::from_x_y_ranges(right - EVAL_BAR_WIDTH..=right, board.y_range());

    // White's share of the bar: 0.5 when equal. `tanh` squeezes any number into -1..1,
    // so +4 pawns already fills most of the bar, and a mate fills all of it.
    let target = 0.5 + 0.5 * (white_cp as f32 / 400.0).tanh();
    // Slides to a new value over a quarter of a second instead of jumping.
    let share = ctx.animate_value_with_time(egui::Id::unique("eval_bar"), target, 0.25);

    // White's part is at White's side of the board: the bottom, unless the board is flipped.
    let white_height = bar.height() * share;
    let white_part = if layout.flipped {
        Rect::from_min_size(bar.min, Vec2::new(bar.width(), white_height))
    } else {
        Rect::from_min_max(Pos2::new(bar.left(), bar.bottom() - white_height), bar.max)
    };
    painter.rect_filled(bar, 4, EVAL_BLACK);
    painter.rect_filled(white_part, 4, EVAL_WHITE);

    // The number goes at the end of the side that is better, so it's always on its color.
    let white_better = white_cp >= 0;
    let at_bottom = white_better != layout.flipped;
    let (pos, align) = if at_bottom {
        (
            bar.center_bottom() - Vec2::new(0.0, 4.0),
            Align2::CENTER_BOTTOM,
        )
    } else {
        (bar.center_top() + Vec2::new(0.0, 4.0), Align2::CENTER_TOP)
    };
    let text_color = if white_better { EVAL_BLACK } else { EVAL_WHITE };
    let text = analysis::format_eval(white_cp);
    painter.text(pos, align, text, FontId::proportional(10.0), text_color);
}

/// Paints the 64 light and dark squares.
fn draw_squares(painter: &Painter, layout: &Layout, board_theme: BoardTheme) {
    for square in Square::all() {
        let color = square_color(square, board_theme);
        painter.rect_filled(
            layout.square_rect(square),
            layout.corner_radius(square),
            color,
        );
    }
}

/// Paints a see-through color over one square.
fn tint_square(painter: &Painter, layout: &Layout, square: Square, color: Color32) {
    painter.rect_filled(
        layout.square_rect(square),
        layout.corner_radius(square),
        color,
    );
}

/// Paints a red glow on the square of a king in check: circles that get smaller towards the
/// middle. They overlap, so the middle is the reddest, like a soft gradient.
fn draw_check_glow(painter: &Painter, layout: &Layout, square: Square) {
    const LAYERS: usize = 8;
    let center = layout.square_rect(square).center();
    for layer in 1..=LAYERS {
        let radius = layout.square_size * 0.6 * layer as f32 / LAYERS as f32;
        painter.circle_filled(center, radius, CHECK_GLOW);
    }
}

/// Writes the file letters along the bottom row and the rank number along the left column.
fn draw_label(painter: &Painter, layout: &Layout, board_theme: BoardTheme) {
    let font = FontId::proportional(layout.square_size * 0.18);
    let padding = Vec2::splat(layout.square_size * 0.05);

    for col in 0..8 {
        let square = layout.square_from_screen(col, 7);
        painter.text(
            layout.square_rect(square).right_bottom() - padding,
            Align2::RIGHT_BOTTOM,
            square.file_char(),
            font.clone(),
            label_color(square, board_theme),
        );
    }

    for row in 0..8 {
        let square = layout.square_from_screen(0, row);
        painter.text(
            layout.square_rect(square).left_top() + padding,
            Align2::LEFT_TOP,
            square.rank + 1,
            font.clone(),
            label_color(square, board_theme),
        );
    }
}

/// Draws every piece as its image, stretched to fill its square.
/// A sliding piece is drawn between its two squares, on top of the others.
/// The `hidden` square's piece is skipped: it is being dragged.
fn draw_pieces(
    painter: &Painter,
    layout: &Layout,
    board: &Board,
    textures: &PieceTextures,
    slide: Option<Slide>,
    hidden: Option<Square>,
) {
    for square in Square::all() {
        let sliding = slide.is_some_and(|slide| slide.to == square);
        if let Some(piece) = board.get(square)
            && !sliding
            && hidden != Some(square)
        {
            let rect = layout.square_rect(square);
            painter.image(textures.get(piece), rect, WHOLE_IMAGE, Color32::WHITE);
        }
    }

    if let Some(slide) = slide
        && let Some(piece) = board.get(slide.to)
    {
        // Fast at first, slow at the end ("ease out") looks more natural than a constant speed.
        let t = 1.0 - (1.0 - slide.progress).powi(2);
        let from = layout.square_rect(slide.from).center();
        let to = layout.square_rect(slide.to).center();
        let rect = Rect::from_center_size(from.lerp(to, t), Vec2::splat(layout.square_size));
        painter.image(textures.get(piece), rect, WHOLE_IMAGE, Color32::WHITE);
    }
}

/// Marks the squares the selected piece can move to: a dot on empty squares, a ring around enemy
/// pieces that can be captured.
fn draw_move_dots(painter: &Painter, layout: &Layout, board: &Board, targets: &[Square]) {
    for &square in targets {
        let center = layout.square_rect(square).center();
        if board.get(square).is_some() {
            // A ring near the edge of the square, so the piece inside stays visible.
            let stroke = Stroke::new(layout.square_size * 0.09, MOVE_DOT);
            painter.circle_stroke(center, layout.square_size * 0.44, stroke);
        } else {
            painter.circle_filled(center, layout.square_size * 0.16, MOVE_DOT);
        }
    }
}

/// Draws an arrow from the center of one square to the center of another.
fn draw_arrow(painter: &Painter, layout: &Layout, from: Square, to: Square, color: Color32) {
    let start = layout.square_rect(from).center();
    let end = layout.square_rect(to).center();
    let stroke = Stroke::new(layout.square_size * 0.12, color);
    painter.arrow(start, end - start, stroke);
}

/// The color of a square on the board.
fn square_color(square: Square, board_theme: BoardTheme) -> Color32 {
    let (light, dark) = theme::board_colors(board_theme);
    if square.is_light() { light } else { dark }
}

/// Labels use the opposite color of their square.
fn label_color(square: Square, board_theme: BoardTheme) -> Color32 {
    let (light, dark) = theme::board_colors(board_theme);
    if square.is_light() { dark } else { light }
}
