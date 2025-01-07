use anyhow::{anyhow, Result};

use nvi::{
    highlights::*,
    input, lua_exec,
    nvi_macros::*,
    nvim::types::{TabPage, Window},
    ui::pane,
};

mod demos;
#[cfg(test)]
mod tests;

const DEFAULT_KEYS: &[&str] = &[
    "a", "s", "d", "f", "g", "h", "j", "k", "l", "q", "w", "e", "r", "t", "y", "u", "i", "o", "p",
    "z", "x", "c", "v", "b", "n", "m",
];

const FLOAT_WIDTH: usize = 7;
const FLOAT_HEIGHT: usize = 3;

#[derive(Clone)]
struct NviWin {
    keys: Vec<String>,
    panes: Vec<pane::Pane>,
}

#[nvi_plugin]
impl NviWin {
    fn new() -> Self {
        NviWin {
            keys: DEFAULT_KEYS.iter().map(|s| s.to_string()).collect(),
            panes: vec![],
        }
    }

    fn highlights(&self) -> Highlights {
        Highlights::default().hl("Window", Hl::default().fg("#ffffff").bg("#215b91"))
    }

    async fn show_hints(&mut self, client: &mut nvi::Client, windows: &[Window]) -> Result<()> {
        for (i, w) in windows.iter().enumerate() {
            let key = self.keys[i].clone();

            let pane = pane::Pane::builder()
                .with_win_pos(w.clone(), pane::Pos::Center, 0)
                .winhl("Normal", "nvi_winWindow")
                .build(
                    client,
                    pane::Content::center(FLOAT_WIDTH, FLOAT_HEIGHT, &key.to_string()),
                )
                .await?;
            self.panes.push(pane);
        }
        lua_exec!(client, "vim.cmd('redraw')").await?;
        Ok(())
    }

    /// Get the list of windows we need to choose from. Exclude the current window and
    /// floating windows.
    async fn windows(&self, client: &mut nvi::Client) -> Result<Vec<Window>> {
        let current = client.nvim.get_current_win().await?;
        let mut ret = vec![];
        for w in client.nvim.tabpage_list_wins(&TabPage::current()).await? {
            if w == current {
                continue;
            }
            let cnf = client.nvim.win_get_config(&w).await?;
            if cnf.relative.is_some() {
                continue;
            }
            ret.push(w);
        }
        Ok(ret)
    }

    /// Pick a window, and return the window ID. If there's only one window, return that window
    /// immediately. Otherwise, display an overlay and ask the user for input.
    #[request]
    async fn pick(&mut self, client: &mut nvi::Client) -> Result<Option<Window>> {
        let windows = self.windows(client).await?;
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
    async fn jump(&mut self, client: &mut nvi::Client) -> Result<()> {
        if let Some(window) = self.pick(client).await? {
            client.nvim.set_current_win(&window).await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    nvi::cmd::run(NviWin::new(), Some(demos::demos())).await?;
    Ok(())
}
