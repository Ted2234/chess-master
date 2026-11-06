use crate::board::Board;
use crate::square::Square;
use crate::ui::piece_textures::PieceTextures;
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Vec2};

const LIGHT_SQUARE: Color32 = Color32::from_rgb(204, 255, 255);
const DARK_SQUARE: Color32 = Color32::from_rgb(0, 102, 204);
const SELECTED: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 0, 90);
const MOVE_DOT: Color32 = Color32::from_rgba_unmultiplied_const(0, 0, 0, 70);

/// Where the board is on the screen and how big each square is.
struct Layout {
    origin: Pos2,
    square_size: f32,
    flipped: bool,
}

impl Layout {
    /// Converts a screen column and row to a board square.
    fn square_from_screen(&self, col: usize, row: usize) -> Square {
        if self.flipped {
            Square::new(row, 7 - col)
        } else {
            Square::new(7 - row, col)
        }
    }

    ///Returns the screen rectangle of a square
    fn square_rect(&self, square: Square) -> Rect {
        let (col, row) = if self.flipped {
            (7 - square.file, square.rank)
        } else {
            (square.file, 7 - square.rank)
        };
        let min = self.origin + Vec2::new(col as f32, row as f32) * self.square_size;
        Rect::from_min_size(min, Vec2::splat(self.square_size))
    }

    /// Returns the square under a screen position, or 'None' if it's outside the board.
    fn square_at(&self, pos: Pos2) -> Option<Square> {
        let x = (pos.x - self.origin.x) / self.square_size;
        let y = (pos.y - self.origin.y) / self.square_size;
        if !(0.0..8.0).contains(&x) || !(0.0..8.0).contains(&y) {
            return None;
        }
        Some(self.square_from_screen(x as usize, y as usize))
    }
}

/// Draws the board and returns the square that was clicked this frame, if any.
pub fn draw_board(
    ui: &mut egui::Ui,
    board: &Board,
    textures: &PieceTextures,
    selected: Option<Square>,
    targets: &[Square],
    flipped: bool,
) -> Option<Square> {
    let size = ui.available_size().min_elem();
    let (response, painter) = ui.allocate_painter(Vec2::splat(size), Sense::click());
    let layout = Layout {
        origin: response.rect.min,
        square_size: size / 8.0,
        flipped,
    };

    draw_squares(&painter, &layout);
    if let Some(square) = selected {
        painter.rect_filled(layout.square_rect(square), 0.0, SELECTED);
    }
    draw_label(&painter, &layout);
    draw_pieces(&painter, &layout, board, textures);
    draw_move_dots(&painter, &layout, board, targets);

    if response.clicked() {
        response
            .interact_pointer_pos()
            .and_then(|pos| layout.square_at(pos))
    } else {
        None
    }
}

/// Paints the 64 light and dark squares.
fn draw_squares(painter: &Painter, layout: &Layout) {
    for _rank in 0..8 {
        for square in Square::all() {
            painter.rect_filled(layout.square_rect(square), 0.0, square_color(square));
        }
    }
}

/// Writes the file letters along the bottom row and the rank number along the left collumn.
fn draw_label(painter: &Painter, layout: &Layout) {
    let font = FontId::proportional(layout.square_size * 0.18);
    let padding = Vec2::splat(layout.square_size * 0.05);

    for col in 0..8 {
        let square = layout.square_from_screen(col, 7);
        let letter = (b'a' + square.file as u8) as char;
        painter.text(
            layout.square_rect(square).right_bottom() - padding,
            Align2::RIGHT_BOTTOM,
            letter,
            font.clone(),
            label_color(square),
        );
    }

    for row in 0..8 {
        let square = layout.square_from_screen(0, row);
        painter.text(
            layout.square_rect(square).left_top() + padding,
            Align2::LEFT_TOP,
            square.rank + 1,
            font.clone(),
            label_color(square),
        );
    }
}

/// Draws every piece as its image, streched to fill its square.
fn draw_pieces(painter: &Painter, layout: &Layout, board: &Board, textures: &PieceTextures) {
    // Which part of the image to use: (0,0) is the top-left rcorner, (1,1) its bottom-right corner.
    let whole_image = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

    for square in Square::all() {
        if let Some(piece) = board.get(square) {
            let rect = layout.square_rect(square);
            painter.image(textures.get(piece), rect, whole_image, Color32::WHITE);
        }
    }
}

/// Marks the squares the selected piece can move to: a dot on empty squares, a ring around enemey
/// pieces that can be captured
fn draw_move_dots(painter: &Painter, layout: &Layout, board: &Board, targets: &[Square]) {
    for &square in targets {
        let center = layout.square_rect(square).center();
        if board.get(square).is_some() {
            let stroke = Stroke::new(layout.square_size * 0.08, MOVE_DOT);
            painter.circle_stroke(center, layout.square_size * 0.04, stroke);
        } else {
            painter.circle_filled(center, layout.square_size * 0.15, MOVE_DOT);
        }
    }
}

/// The color of a squae on the bord.
fn square_color(square: Square) -> Color32 {
    if square.is_light() {
        LIGHT_SQUARE
    } else {
        DARK_SQUARE
    }
}

/// Labels use the opposite color of their square.
fn label_color(square: Square) -> Color32 {
    if square.is_light() {
        DARK_SQUARE
    } else {
        LIGHT_SQUARE
    }
}
