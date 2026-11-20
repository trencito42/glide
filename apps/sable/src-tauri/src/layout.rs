#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub chrome: Rect,
    pub primary: Rect,
    pub secondary: Option<Rect>,
    pub overlay: Option<Rect>,
}

pub const HORIZONTAL_CHROME: f64 = 104.0;
pub const SIDEBAR_WIDTH: f64 = 252.0;
pub const SPLIT_GAP: f64 = 6.0;
pub const OVERLAY_WIDTH: f64 = 560.0;
pub const OVERLAY_HEIGHT: f64 = 420.0;

pub const PARKED: Rect = Rect {
    x: -20000.0,
    y: -20000.0,
    width: 2.0,
    height: 2.0,
};

pub fn frame(window_width: f64, window_height: f64, sidebar: bool, split_ratio: Option<f64>, overlay: bool) -> Frame {
    let width = window_width.max(1.0);
    let height = window_height.max(1.0);
    let (chrome, content) = if sidebar {
        let chrome_width = SIDEBAR_WIDTH.min(width * 0.42).max(1.0);
        (
            Rect { x: 0.0, y: 0.0, width: chrome_width, height },
            Rect {
                x: chrome_width,
                y: 0.0,
                width: (width - chrome_width).max(1.0),
                height,
            },
        )
    } else {
        let chrome_height = HORIZONTAL_CHROME.min(height * 0.4).max(1.0);
        (
            Rect { x: 0.0, y: 0.0, width, height: chrome_height },
            Rect {
                x: 0.0,
                y: chrome_height,
                width,
                height: (height - chrome_height).max(1.0),
            },
        )
    };

    let (primary, secondary) = if let Some(ratio) = split_ratio {
        let ratio = ratio.clamp(0.25, 0.75);
        let gap = if content.width > SPLIT_GAP + 2.0 { SPLIT_GAP } else { 0.0 };
        let usable = (content.width - gap).max(2.0);
        let primary_width = (usable * ratio).clamp(1.0, usable - 1.0);
        let secondary_width = (usable - primary_width).max(1.0);
        (
            Rect {
                x: content.x,
                y: content.y,
                width: primary_width,
                height: content.height,
            },
            Some(Rect {
                x: content.x + primary_width + gap,
                y: content.y,
                width: secondary_width,
                height: content.height,
            }),
        )
    } else {
        (content, None)
    };

    let overlay_rect = if overlay {
        Some(centered(content, OVERLAY_WIDTH, OVERLAY_HEIGHT))
    } else {
        None
    };

    Frame {
        chrome,
        primary,
        secondary,
        overlay: overlay_rect,
    }
}

fn centered(content: Rect, width: f64, height: f64) -> Rect {
    let width = width.min(content.width).max(1.0);
    let height = height.min(content.height).max(1.0);
    Rect {
        x: content.x + (content.width - width) / 2.0,
        y: content.y + (content.height - height) / 2.0,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_split_covers_the_content_without_overlap() {
        let frame = frame(1280.0, 840.0, false, Some(0.5), false);
        let secondary = frame.secondary.expect("split");
        assert!(frame.primary.y >= frame.chrome.y + frame.chrome.height - 0.1);
        assert!(secondary.x >= frame.primary.x + frame.primary.width - 0.1);
        assert!((frame.primary.width - secondary.width).abs() < 2.0);
        let covered = frame.primary.width + secondary.width + SPLIT_GAP;
        assert!((covered - frame.chrome.width).abs() < 1.0);
    }

    #[test]
    fn sidebar_puts_pages_to_the_right_of_chrome() {
        let frame = frame(1280.0, 840.0, true, None, false);
        assert_eq!(frame.chrome.x, 0.0);
        assert!(frame.primary.x >= frame.chrome.width - 0.1);
        assert!(frame.secondary.is_none());
    }

    #[test]
    fn overlay_stays_inside_the_content_rect() {
        let frame = frame(900.0, 600.0, false, None, true);
        let overlay = frame.overlay.expect("overlay");
        assert!(overlay.x >= frame.primary.x - 0.1);
        assert!(overlay.y >= frame.primary.y - 0.1);
        assert!(overlay.x + overlay.width <= frame.primary.x + frame.primary.width + 0.1);
        assert!(overlay.width <= frame.primary.width + 0.1);
    }
}
