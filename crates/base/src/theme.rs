// Modified for GPUI Design: retain only input colors, caret, and scrollbar defaults.
use gpui::{App, Global, Hsla, hsla};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ScrollbarMode, ScrollbarMotion, ScrollbarStyles, input::CursorBlink};

/// Application-wide defaults for Base behavior modules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThemeAppearance {
    #[default]
    Light,
    Dark,
}

/// Baseline colors for text editing, independent of any application design system.
///
/// Explicit colors in [`crate::InputEditorStyle`] take precedence. These defaults
/// make unconfigured input text and its caret visible on a light surface; they
/// do not style a control frame, choose typography, or set layout dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct InputColors {
    pub foreground: Hsla,
    pub muted_foreground: Hsla,
    pub background: Hsla,
    pub border: Hsla,
    pub selection: Hsla,
}

impl Default for InputColors {
    fn default() -> Self {
        Self {
            foreground: hsla(0., 0., 0.039, 1.),
            muted_foreground: hsla(0., 0., 0.451, 1.),
            background: hsla(0., 0., 1., 1.),
            border: hsla(0., 0., 0.898, 1.),
            selection: hsla(0., 0., 0.961, 0.4),
        }
    }
}

/// Construct with [`Theme::default`] and assign fields; new fields may be
/// added without a breaking change.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct Theme {
    pub appearance: ThemeAppearance,
    /// Baseline colors used when an input has no explicit projected color.
    pub input_colors: InputColors,
    pub scrollbar: ScrollbarTheme,
    /// Application-wide caret blink behavior.
    pub cursor_blink: CursorBlink,
}

impl Global for Theme {}

impl Theme {
    pub fn global(cx: &App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
        cx.global_mut::<Self>()
    }
}

/// Access to the active base theme through an application context.
pub(crate) trait ActiveTheme {
    fn theme(&self) -> Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> Theme {
        Theme::global(self)
    }
}

/// Global defaults used by [`crate::Scrollbar`].
///
/// `motion` defaults to motionless. Styled layers project their own timing;
/// Base never installs a fade or slide of its own.
#[derive(Clone, Default)]
pub struct ScrollbarTheme {
    mode: ScrollbarMode,
    motion: ScrollbarMotion,
    styles: ScrollbarStyles,
}

impl ScrollbarTheme {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mode(mut self, mode: ScrollbarMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_motion(mut self, motion: ScrollbarMotion) -> Self {
        self.motion = motion;
        self
    }

    pub fn with_styles(mut self, styles: ScrollbarStyles) -> Self {
        self.styles = styles;
        self
    }

    pub fn mode(&self) -> ScrollbarMode {
        self.mode
    }

    pub fn motion(&self) -> ScrollbarMotion {
        self.motion
    }

    pub fn styles(&self) -> &ScrollbarStyles {
        &self.styles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_theme_supports_builder_construction() {
        let mode = ScrollbarMode::Hover;
        let motion = ScrollbarMotion::default().with_enter(std::time::Duration::from_millis(120));
        let styles = ScrollbarStyles::default();
        let theme = ScrollbarTheme::new()
            .with_mode(mode)
            .with_motion(motion)
            .with_styles(styles);

        assert_eq!(theme.mode(), mode);
        assert_eq!(theme.motion(), motion);
        let _ = theme.styles();
    }
}
