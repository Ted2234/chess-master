use eframe::{
    egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Vec2},
    wgpu::Origin2d,
};

use crate::board::Board;

const LIGHT_SQUARE: Color32 = Color32::from_rgb(204, 255, 255);
const DARK_SQUARE: Color32 = Color32::from_rgb(0, 102, 204);

///Draw the board, as a large as the available space allows.
pub fn draw_board(ui: &mut egui::Ui, board: &Board) {
    let size = ui.available_size().min_elem();
    let (response, painter) = ui.allocate_painter(Vec2::splat(size), Sense::click());
    let origin = response.rect.min;
    let square_size = size / 8.0;

    draw_squares(&painter, origin, square_size);
    draw_labels(&painter, origin, square_size);
    draw_pieces(&painter, origin, square_size, board);
}

/// Paint the 64 light and dark squares.
fn draw_squares(painter: &Painter, origin: Pos2, square_size: f32) {
    for rank in 0..8 {
        for file in 0..8 {
            let rect = square_rect(origin, square_size, rank, file);
            painter.rect_filled(rect, 0.0, square_color(rank, file));
        }
    }
}

/// Writes the file letters (a-h) and rank numbers (1-8) inside the edge squares.
fn draw_labels(painter: &Painter, origin: Pos2, square_size: f32) {
    let font = FontId::proportional(square_size * 0.18);
    let padding = square_size * 0.05;

    for file in 0..8 {
        let rect = square_rect(origin, square_size, 0, file);
        let letter = (b'a' + file as u8) as char;
        painter.text(
            rect.right_bottom() + Vec2::new(-padding, -padding),
            Align2::RIGHT_BOTTOM,
            letter,
            font.clone(),
            square_color(0, file + 1),
        );
    }

    for rank in 0..8 {
        let rect = square_rect(origin, square_size, rank, 0);
        painter.text(
            rect.left_top() + Vec2::new(padding, padding),
            Align2::LEFT_TOP,
            rank + 1,
            font.clone(),
            square_color(rank + 1, 0),
        );
    }
}

/// Draw every piece as its letter (temporary)
fn draw_pieces(painter: &Painter, origin: Pos2, square_size: f32, board: &Board) {
    let font = FontId::proportional(square_size * 0.6);

    for rank in 0..8 {
        for file in 0..8 {
            if let Some(piece) = board.get(rank, file) {
                let rect = square_rect(origin, square_size, rank, file);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    piece.symbol(),
                    font.clone(),
                    Color32::BLACK,
                );
            }
        }
    }
}

/// Returns the screen rectangle of square. Rank 0 (rank 1 is at the bottom)
fn square_rect(origin: Pos2, square_size: f32, rank: usize, file: usize) -> Rect {
    let x = origin.x + file as f32 * square_size;
    let y = origin.y + (7 - rank) as f32 * square_size;
    Rect::from_min_size(Pos2::new(x, y), Vec2::splat(square_size))
}

/// g1 is a dark square, and colors alternate from there
fn square_color(rank: usize, file: usize) -> Color32 {
    if (rank + file).is_multiple_of(2) {
        DARK_SQUARE
    } else {
        LIGHT_SQUARE
    }
}
