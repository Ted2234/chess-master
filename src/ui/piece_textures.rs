use std::collections::HashMap;

use eframe::egui::{self, ColorImage, TextureHandle, TextureId, TextureOptions};

use crate::piece::{Color, Piece, PieceKind};

/// The images of all 12 pieces, loaded once when the app starts.
pub struct PieceTextures {
    textures: HashMap<Piece, TextureHandle>,
}

impl PieceTextures {
    /// Decodes every piece image and uploads it to the graphics card.
    pub fn load(ctx: &egui::Context) -> Result<Self, image::ImageError> {
        let mut textures = HashMap::new();
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                let piece = Piece::new(color, kind);
                let image = decode_png(image_bytes(piece))?;
                let name = format!("{color:?}_{kind:?}");

                // NEAREST keeps the pixel art sharp.
                let texture = ctx.load_texture(name, image, TextureOptions::NEAREST);
                textures.insert(piece, texture);
            }
        }
        Ok(Self { textures })
    }

    /// Returns the texture to draw for a piece.
    pub fn get(&self, piece: Piece) -> TextureId {
        self.textures[&piece].id()
    }
}

/// Turns the bytes of a PNG file into an image egui can use.
fn decode_png(bytes: &[u8]) -> Result<ColorImage, image::ImageError> {
    let rgba = image::load_from_memory(bytes)?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// The PNG file of each piece, built into the program at compile time.
fn image_bytes(piece: Piece) -> &'static [u8] {
    match (piece.color, piece.kind) {
        (Color::White, PieceKind::King) => include_bytes!("../../assets/pieces/white_king.png"),
        (Color::White, PieceKind::Queen) => include_bytes!("../../assets/pieces/white_queen.png"),
        (Color::White, PieceKind::Rook) => include_bytes!("../../assets/pieces/white_rook.png"),
        (Color::White, PieceKind::Bishop) => include_bytes!("../../assets/pieces/white_bishop.png"),
        (Color::White, PieceKind::Knight) => include_bytes!("../../assets/pieces/white_knight.png"),
        (Color::White, PieceKind::Pawn) => include_bytes!("../../assets/pieces/white_pawn.png"),
        (Color::Black, PieceKind::King) => include_bytes!("../../assets/pieces/black_king.png"),
        (Color::Black, PieceKind::Queen) => include_bytes!("../../assets/pieces/black_queen.png"),
        (Color::Black, PieceKind::Rook) => include_bytes!("../../assets/pieces/black_rook.png"),
        (Color::Black, PieceKind::Bishop) => include_bytes!("../../assets/pieces/black_bishop.png"),
        (Color::Black, PieceKind::Knight) => include_bytes!("../../assets/pieces/black_knight.png"),
        (Color::Black, PieceKind::Pawn) => include_bytes!("../../assets/pieces/black_pawn.png"),
    }
}
