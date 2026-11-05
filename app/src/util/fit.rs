use crownui::prelude::{Point, Rect, Size};

/// `content` scaled to the largest size that fits inside `bounds`, centered.
pub fn fitted(content: Size, bounds: Size) -> Rect {
    if content.width <= 0.0 || content.height <= 0.0 {
        return Rect::new(Point::ZERO, bounds);
    }
    let scale = (bounds.width / content.width).min(bounds.height / content.height);
    let size = Size::new(content.width * scale, content.height * scale);
    let origin = Point::new(
        (bounds.width - size.width) / 2.0,
        (bounds.height - size.height) / 2.0,
    );
    Rect::new(origin, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_content_fits_by_width() {
        let rect = fitted(Size::new(400.0, 100.0), Size::new(200.0, 200.0));
        assert_eq!(
            rect,
            Rect::new(Point::new(0.0, 75.0), Size::new(200.0, 50.0))
        );
    }

    #[test]
    fn tall_content_fits_by_height() {
        let rect = fitted(Size::new(90.0, 160.0), Size::new(320.0, 80.0));
        assert_eq!(
            rect,
            Rect::new(Point::new(137.5, 0.0), Size::new(45.0, 80.0))
        );
    }
}
