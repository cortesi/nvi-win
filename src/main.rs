use anyhow::Result;

use nvi::{
    highlights::*,
    input,
    nvi_macros::*,
    nvim::types::{TabPage, Window},
    ui::pane,
    Color,
};

mod demos;
#[cfg(test)]
mod tests;

const DEFAULT_KEYS: &str = "asdfghjklqwertyuiopzxcvbnm";
const FLOAT_WIDTH: usize = 7;
const FLOAT_HEIGHT: usize = 3;

enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// Find the next window in the given direction. Directions are calculated relative to the top-left
/// corner of the window. If there is no window in the given direction, return None. The window
/// layout may have gaps and overlaps.
///
/// `current` is the index of the current window in the `geoms` vector.
/// `geoms` is a slice of tuples containing the geometry of each window in the layout as (x, y,
/// width, height) tuples, with x and y being the coordinates of the top-left corner of the window.
fn find_dir(dir: Dir, current: usize, geoms: &[(i64, i64, i64, i64)]) -> Option<usize> {
    let &(curr_x, curr_y, curr_w, curr_h) = geoms.get(current)?;
    let curr_right = curr_x + curr_w;
    let curr_bottom = curr_y + curr_h;

    let mut candidates = Vec::new();

    for (i, &(x, y, w, h)) in geoms.iter().enumerate() {
        if i == current {
            continue;
        }

        let right = x + w;
        let bottom = y + h;

        match dir {
            Dir::Right => {
                if x > curr_x && ranges_overlap(curr_y, curr_bottom, y, bottom) {
                    candidates.push((i, x - curr_right));
                }
            }
            Dir::Left => {
                if right < curr_right && ranges_overlap(curr_y, curr_bottom, y, bottom) {
                    candidates.push((i, curr_x - right));
                }
            }
            Dir::Down => {
                if y > curr_y && ranges_overlap(curr_x, curr_right, x, right) {
                    candidates.push((i, y - curr_bottom));
                }
            }
            Dir::Up => {
                if bottom < curr_bottom && ranges_overlap(curr_x, curr_right, x, right) {
                    candidates.push((i, curr_y - bottom));
                }
            }
        }
    }

    // Sort by distance and return the closest window
    candidates.sort_by_key(|&(_, dist)| dist);
    candidates.first().map(|&(idx, _)| idx)
}

/// Returns true if two ranges [start1, end1) and [start2, end2) overlap.
fn ranges_overlap(start1: i64, end1: i64, start2: i64, end2: i64) -> bool {
    start1 < end2 && start2 < end1
}

struct NviWin {
    keys: Vec<String>,
    panes: Vec<pane::Pane>,
}

#[nvi_plugin]
/// A window navigation plugin.
///
/// A key feature of the plugin is the fact that it ignores non-floating windows with `focusable`
/// set to false. This makes non-floating interface panes for plugins possible. See the following
/// neovim tracking issue for more information:
///
/// https://github.com/neovim/neovim/issues/29365
impl NviWin {
    fn new() -> Self {
        NviWin {
            keys: DEFAULT_KEYS.chars().map(|c| c.to_string()).collect(),
            panes: vec![],
        }
    }

    fn highlights(&self) -> nvi::error::Result<Highlights> {
        Ok(Highlights::default().hl(
            "Window",
            Hl::default().fg(Color::White)?.bg(Color::AzureBlue)?,
        ))
    }

    async fn show_hints(
        &mut self,
        client: &mut nvi::Client,
        windows: &[Window],
    ) -> nvi::error::Result<()> {
        for (i, w) in windows.iter().enumerate() {
            let key = self.keys[i].clone();

            let pane = pane::Pane::builder()
                .with_win_pos(w.clone(), pane::Pos::Center, 0)
                .winhl("Normal", &client.hl_name("Window")?)
                .build(
                    client,
                    pane::Text::center(FLOAT_WIDTH, FLOAT_HEIGHT, &key.to_string()),
                )
                .await?;
            self.panes.push(pane);
        }
        client.redraw().await?;
        Ok(())
    }

    /// Get the list of windows we need to choose from. Exclude floating windows, and windows with
    /// focusable set to false. Windows are returned in layout order.
    async fn windows(&self, client: &mut nvi::Client) -> nvi::error::Result<Vec<Window>> {
        let mut ret = vec![];
        for w in client.nvim.tabpage_list_wins(&TabPage::current()).await? {
            let cnf = client.nvim.win_get_config(&w).await?;
            if cnf.focusable == Some(false) || cnf.relative.is_some() {
                continue;
            }
            ret.push(w);
        }
        Ok(ret)
    }

    #[allow(dead_code)]
    pub async fn geoms(
        &self,
        client: &mut nvi::Client,
        windows: &[Window],
    ) -> nvi::error::Result<Vec<(i64, i64, i64, i64)>> {
        let mut ret = vec![];
        for w in windows {
            ret.push(w.geom(client).await?);
        }
        Ok(ret)
    }

    /// Visually pick a window, and return the window ID. If there's only one window, return that
    /// window immediately. Otherwise, display an overlay and ask the user for input. If the user
    /// presses any key not in our shortcut list, cancel the pick operation and return None.
    #[request]
    async fn pick(&mut self, client: &mut nvi::Client) -> nvi::error::Result<Option<Window>> {
        let current = client.nvim.get_current_win().await?;
        let windows = self
            .windows(client)
            .await?
            .into_iter()
            .filter(|w| *w != current)
            .collect::<Vec<_>>();

        self.show_hints(client, &windows).await?;
        let c = input::get_keypress(client).await?;

        while let Some(p) = self.panes.pop() {
            p.destroy(client).await?;
        }

        if let Some(offset) = self.keys.iter().position(|x| **x == c.key.name()) {
            if offset < windows.len() {
                Ok(Some(windows[offset].clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Pick a window and jump to it.
    #[request]
    async fn jump(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        if let Some(window) = self.pick(client).await? {
            client.nvim.set_current_win(&window).await?;
        }
        Ok(())
    }

    /// Helper function to get current window index and window list
    async fn get_window_info(
        &self,
        client: &mut nvi::Client,
    ) -> nvi::error::Result<(usize, Vec<Window>)> {
        let windows = self.windows(client).await?;
        let current = client.nvim.get_current_win().await?;
        let offset = windows.iter().position(|w| *w == current).unwrap();
        Ok((offset, windows))
    }

    /// Helper function to move to a target window
    async fn move_to_window(
        &self,
        client: &mut nvi::Client,
        window: &Window,
    ) -> nvi::error::Result<()> {
        client.nvim.set_current_win(window).await?;
        Ok(())
    }

    /// Go to the next window in the layout order, wrapping if needed.
    #[request]
    async fn next(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        let (offset, windows) = self.get_window_info(client).await?;
        let next = windows[(offset + 1) % windows.len()].clone();
        self.move_to_window(client, &next).await
    }

    /// Go to the previous window in the layout order, wrapping if needed.
    #[request]
    async fn prev(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        let (offset, windows) = self.get_window_info(client).await?;
        let prev = windows[(offset + windows.len() - 1) % windows.len()].clone();
        self.move_to_window(client, &prev).await
    }

    async fn move_to_dir(&mut self, dir: Dir, client: &mut nvi::Client) -> nvi::error::Result<()> {
        let (offset, windows) = self.get_window_info(client).await?;
        let geoms = self.geoms(client, &windows).await?;
        if let Some(idx) = find_dir(dir, offset, &geoms) {
            self.move_to_window(client, &windows[idx]).await?;
        }
        Ok(())
    }

    /// Go to the window to the left of the current window.
    #[request]
    async fn left(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        self.move_to_dir(Dir::Left, client).await
    }

    /// Go to the window to the right of the current window.
    #[request]
    async fn right(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        self.move_to_dir(Dir::Right, client).await
    }

    /// Go to the window above the current window.
    #[request]
    async fn up(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        self.move_to_dir(Dir::Up, client).await
    }

    /// Go to the window below the current window.
    #[request]
    async fn down(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        self.move_to_dir(Dir::Down, client).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    nvi::cmd::run(NviWin::new(), Some(demos::demos())).await?;
    Ok(())
}
