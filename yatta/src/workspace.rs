use std::mem;

use bindings::windows::{
    win32::{
        display_devices::POINT,
        gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO},
        menus_and_resources::GetCursorPos,
        system_services::{HWND_TOP, MONITOR_DEFAULTTONEAREST, SWP_NOMOVE, SWP_NOSIZE},
        windows_and_messaging::{EnumWindows, HWND, LPARAM},
    },
    BOOL,
};
use yatta_core::Layout;

use crate::{rect::Rect, window::Window, DirectionOperation};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub dimensions:        Rect,
    pub windows:           Vec<Window>,
    pub layout_dimensions: Vec<Rect>,
    pub layout:            Layout,
    pub foreground_window: Window,
    pub gaps:              i32,
    pub paused:            bool,
}

pub const PADDING: i32 = 20;

impl Workspace {
    pub fn get_dimensions(&mut self) {
        let active_monitor = unsafe {
            let mut cursor_pos: POINT = mem::zeroed();
            GetCursorPos(&mut cursor_pos);

            MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST as u32)
        };

        let mut rect: Rect = unsafe {
            let mut info: MONITORINFO = mem::zeroed();
            info.cb_size = mem::size_of::<MONITORINFO>() as u32;

            GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFO as *mut _);

            info.rc_work.into()
        };

        rect.height -= PADDING * 2;
        rect.width -= PADDING * 2;
        rect.y += PADDING;
        rect.x += PADDING;

        self.dimensions = rect;
    }

    pub fn get_visible_windows(&mut self) {
        self.windows.clear();

        unsafe {
            EnumWindows(
                Some(enum_window),
                LPARAM(&mut self.windows as *mut Vec<Window> as isize),
            );
        }
    }

    pub fn get_foreground_window(&mut self) {
        self.foreground_window = Window::foreground();
    }

    pub fn get_foreground_window_index(&mut self) -> usize {
        let mut idx = 0;

        for (i, w) in self.windows.iter().enumerate() {
            if self.foreground_window.hwnd == w.hwnd {
                idx = i;
                break;
            }
        }

        idx
    }

    pub fn follow_focus_with_mouse(&mut self, idx: usize) {
        let window = self.windows.get(idx).unwrap();
        window.set_cursor_pos(self.layout_dimensions[idx]);
    }

    pub fn window_op_up(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 2 && idx != 0 && idx != 1,
            Layout::BSPH => self.windows.len() > 1 && idx != 0,
            Layout::Columns | Layout::Monocle => false,
            Layout::Rows => idx != 0,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }

                Layout::BSPH => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }
                Layout::Columns | Layout::Monocle => unreachable!(),
                Layout::Rows => idx - 1,
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_down(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let len = self.windows.len();

        let can_move = match self.layout {
            Layout::BSPV => len > 2 && idx != len - 1 && idx % 2 != 0,
            Layout::BSPH => self.windows.len() > 1 && idx % 2 == 0,
            Layout::Columns | Layout::Monocle => false,
            Layout::Rows => idx != self.windows.len() - 1,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV | Layout::BSPH | Layout::Rows => idx + 1,
                Layout::Columns | Layout::Monocle => unreachable!(),
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_left(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 1 && idx != 0,
            Layout::BSPH => self.windows.len() > 2 && idx != 0 && idx != 1,
            Layout::Columns => idx != 0,
            Layout::Rows | Layout::Monocle => false,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }

                Layout::BSPH => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }

                Layout::Columns => idx - 1,
                Layout::Rows | Layout::Monocle => unreachable!(),
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_right(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();

        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 1 && idx % 2 == 0,
            Layout::BSPH => self.windows.len() > 2 && idx % 2 != 0 && idx != self.windows.len() - 1,
            Layout::Columns => idx != self.windows.len() - 1,
            Layout::Rows | Layout::Monocle => false,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV | Layout::BSPH | Layout::Columns => idx + 1,
                Layout::Rows | Layout::Monocle => unreachable!(),
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_next(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1;

        if can_move {
            let new_idx = if idx == self.windows.len() - 1 {
                0
            } else {
                idx + 1
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_previous(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1;

        if can_move {
            let new_idx = if idx == 0 {
                self.windows.len() - 1
            } else {
                idx - 1
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn calculate_layout(&mut self) {
        let len = self.windows.iter().filter(|x| x.should_tile).count();
        match self.layout {
            Layout::Monocle => self.layout_dimensions = bsp(0, 1, self.dimensions, 1, self.gaps),
            Layout::BSPV => {
                self.layout_dimensions = bsp(0, len, self.dimensions, 1, self.gaps);
            }
            Layout::BSPH => {
                self.layout_dimensions = bsp(0, len, self.dimensions, 0, self.gaps);
            }
            Layout::Columns => {
                let width_f = self.dimensions.width as f32 / self.windows.len() as f32;
                let width = width_f.floor() as i32;

                let mut x = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x: self.dimensions.x + x,
                        y: self.dimensions.y,
                        width,
                        height: self.dimensions.height,
                    });
                    x += width;
                }
                self.layout_dimensions = layouts
            }
            Layout::Rows => {
                let height_f = self.dimensions.height as f32 / self.windows.len() as f32;
                let height = height_f.floor() as i32;

                let mut y = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x: self.dimensions.x,
                        y: self.dimensions.y + y,
                        width: self.dimensions.width,
                        height,
                    });
                    y += height;
                }
                self.layout_dimensions = layouts
            }
        }
    }

    pub fn apply_layout(&mut self, new_focus: Option<usize>) {
        if let Layout::Monocle = self.layout {
            self.get_foreground_window();
            self.foreground_window
                .set_pos(self.layout_dimensions[0], Option::from(HWND_TOP), None);

            return;
        }

        let mut skipped = 0;
        for (i, w) in self.windows.iter().enumerate() {
            if w.should_tile {
                if let Some(new_idx) = new_focus {
                    // Make sure this is focused
                    if i == new_idx {
                        w.set_pos(
                            self.layout_dimensions[new_idx],
                            Option::from(HWND_TOP),
                            Option::from(SWP_NOMOVE as u32 | SWP_NOSIZE as u32),
                        );
                    } else {
                        w.set_pos(self.layout_dimensions[i - skipped], None, None)
                    }
                } else {
                    w.set_pos(self.layout_dimensions[i - skipped], None, None)
                }
            } else {
                skipped += 1
            }
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        let mut workspace = Workspace {
            dimensions:        Rect::zero(),
            windows:           vec![],
            layout_dimensions: vec![],
            layout:            Layout::BSPV,
            foreground_window: Window::default(),
            gaps:              5,
            paused:            false,
        };

        workspace.get_dimensions();
        workspace.get_visible_windows();
        workspace.get_foreground_window();
        workspace.calculate_layout();
        workspace.apply_layout(None);

        workspace
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<Window>) };

    let w = Window {
        hwnd,
        should_tile: true,
    };

    if w.is_visible() && !w.is_minimized() && w.should_manage(None) {
        windows.push(w)
    }

    true.into()
}

fn bsp(i: usize, window_count: usize, area: Rect, vertical: usize, gaps: i32) -> Vec<Rect> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width - gaps * 2,
            height: area.height - gaps * 2,
        }]
    } else if i % 2 == vertical {
        let mut res = vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width - gaps * 2,
            height: area.height / 2 - gaps * 2,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x,
                y:      area.y + area.height / 2,
                width:  area.width,
                height: area.height / 2,
            },
            vertical,
            gaps,
        ));
        res
    } else {
        let mut res = vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width / 2 - gaps * 2,
            height: area.height - gaps * 2,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x + area.width / 2,
                y:      area.y,
                width:  area.width / 2,
                height: area.height,
            },
            vertical,
            gaps,
        ));
        res
    }
}
