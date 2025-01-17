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

struct NviWin {
    keys: Vec<String>,
    panes: Vec<pane::Pane>,
}

#[nvi_plugin]
/// A window navigation plugin.
///
/// This pulugin ignores non-floating window with `focusable` set to false. This makes non-floating
/// interface panes possible, and opens new avenues to explore for plugin interfaces. See the
/// following neovim tracking issue for more information:
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

    /// Pick a window, and return the window ID. If there's only one window, return that window
    /// immediately. Otherwise, display an overlay and ask the user for input. If the user presses
    /// any key not in our shortcut list, cancel the pick operation and return None.
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

    /// Go to the next window, using the layout order of windows, wrapping if needed.
    #[request]
    async fn next(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        let windows = self.windows(client).await?;
        let current = client.nvim.get_current_win().await?;
        let offset = windows.iter().position(|w| *w == current).unwrap();
        let next = windows[(offset + 1) % windows.len()].clone();
        client.nvim.set_current_win(&next).await?;
        Ok(())
    }

    /// Go to the previous window, using the layout order of windows, wrapping if needed.
    #[request]
    async fn prev(&mut self, client: &mut nvi::Client) -> nvi::error::Result<()> {
        let windows = self.windows(client).await?;
        let current = client.nvim.get_current_win().await?;
        let offset = windows.iter().position(|w| *w == current).unwrap();
        let prev = windows[(offset + windows.len() - 1) % windows.len()].clone();
        client.nvim.set_current_win(&prev).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    nvi::cmd::run(NviWin::new(), Some(demos::demos())).await?;
    Ok(())
}
