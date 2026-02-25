pub mod font;
pub mod tiles;
pub mod oam;
pub mod registers;
pub mod disasm;

use crate::gameboy::GameBoy;
use minifb::{Window, Key, KeyRepeat};

// Color constants (0x00RRGGBB)
pub const BG_COLOR: u32      = 0x001A1A2E;
pub const TEXT_COLOR: u32    = 0x00E0E0E0;
pub const HEADER_COLOR: u32  = 0x0000FF88;
pub const HIGHLIGHT_COLOR: u32 = 0x00FFAA00;
pub const BP_COLOR: u32      = 0x00FF4444;

#[allow(dead_code)]
pub enum DebugAction {
    Step,
    BreakpointHit,
}

pub struct DebugWindows {
    pub tile_viewer: Option<tiles::TileViewer>,
    pub oam_viewer: Option<oam::OamViewer>,
    pub register_viewer: Option<registers::RegisterViewer>,
}

impl DebugWindows {
    pub fn new() -> Self {
        DebugWindows {
            tile_viewer: None,
            oam_viewer: None,
            register_viewer: None,
        }
    }

    /// Handle F1/F2/F3 toggle keys from the main window.
    pub fn handle_toggles(&mut self, main_window: &Window) {
        if main_window.is_key_pressed(Key::F1, KeyRepeat::No) {
            if self.tile_viewer.is_some() {
                self.tile_viewer = None;
            } else {
                self.tile_viewer = Some(tiles::TileViewer::new());
            }
        }
        if main_window.is_key_pressed(Key::F2, KeyRepeat::No) {
            if self.oam_viewer.is_some() {
                self.oam_viewer = None;
            } else {
                self.oam_viewer = Some(oam::OamViewer::new());
            }
        }
        if main_window.is_key_pressed(Key::F3, KeyRepeat::No) {
            if self.register_viewer.is_some() {
                self.register_viewer = None;
            } else {
                self.register_viewer = Some(registers::RegisterViewer::new());
            }
        }
    }

    /// Update all open debug windows. Returns an optional DebugAction.
    pub fn update(&mut self, gb: &GameBoy, palette: &[u32; 4]) -> Option<DebugAction> {
        // Close windows that user has closed via X button
        if let Some(ref tv) = self.tile_viewer {
            if !tv.is_open() { self.tile_viewer = None; }
        }
        if let Some(ref ov) = self.oam_viewer {
            if !ov.is_open() { self.oam_viewer = None; }
        }
        if let Some(ref rv) = self.register_viewer {
            if !rv.is_open() { self.register_viewer = None; }
        }

        // Update tile viewer
        if let Some(ref mut tv) = self.tile_viewer {
            tv.update(
                &gb.cpu.bus.vram,
                gb.cpu.bus.ppu.bgp,
                palette,
            );
        }

        // Update OAM viewer
        if let Some(ref mut ov) = self.oam_viewer {
            ov.update(
                &gb.cpu.bus.vram,
                &gb.cpu.bus.oam,
                gb.cpu.bus.ppu.obp0,
                gb.cpu.bus.ppu.obp1,
                palette,
            );
        }

        // Update register viewer
        let mut action = None;
        if let Some(ref mut rv) = self.register_viewer {
            action = rv.update(gb, palette);
        }

        action
    }

    /// Returns breakpoints from the register viewer (if open).
    pub fn breakpoints(&self) -> Option<&std::collections::HashSet<u16>> {
        self.register_viewer.as_ref().map(|rv| &rv.breakpoints)
    }

    #[allow(dead_code)]
    pub fn any_open(&self) -> bool {
        self.tile_viewer.is_some() || self.oam_viewer.is_some() || self.register_viewer.is_some()
    }
}
