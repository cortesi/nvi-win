use anyhow::{anyhow, Result};
use nvi::nvi_macros::*;
use nvi::types::{Border, Relative, TabPage, Window, WindowConf};
use tracing::debug;

#[cfg(test)]
mod tests;

const DEFAULT_HINT_HL: &str = "Bold";
const DEFAULT_NORMAL_HL: &str = "Normal";
const DEFAULT_KEYS: &str = "asdfghjklqwertyuiopzxcvbnm";
const FLOAT_WIDTH: i64 = 6;
const FLOAT_HEIGHT: i64 = 3;

#[derive(Clone)]
struct NviWin {
    keys: String,
}

#[nvi_service]
impl NviWin {
    fn new() -> Self {
        NviWin {
            keys: DEFAULT_KEYS.to_string(),
        }
    }

    async fn show_hints(&self, client: &mut nvi::Client, windows: Vec<Window>) -> Result<()> {
        for (i, w) in windows.iter().enumerate() {
            let key = self.keys.chars().nth(i).unwrap();
            let buffer = client.nvim.create_buf(false, true).await?;
            if u64::from(buffer.clone()) == 0 {
                return Err(anyhow!("failed to create buffer"));
            }
            client
                .nvim
                .buf_set_lines(
                    &buffer,
                    0,
                    -1,
                    false,
                    vec![
                        "".to_string(),
                        "   ".to_string(),
                        key.to_string(),
                        "   ".to_string(),
                        "".to_string(),
                    ],
                )
                .await?;
            client
                .nvim
                .buf_add_highlight(&buffer, 0, DEFAULT_HINT_HL, 1, 0, -1)
                .await?;

            let width = client.nvim.win_get_width(w).await?;
            let height = client.nvim.win_get_height(w).await?;
            let row = (height - FLOAT_HEIGHT) / 2 - 1;
            let col = (width - FLOAT_WIDTH) / 2 - FLOAT_WIDTH;

            let float_win = client
                .nvim
                .open_win(
                    &buffer,
                    true,
                    WindowConf::default()
                        .relative(Relative::Win)
                        .win(w.clone())
                        .row(row as u64)
                        .col(col as u64)
                        .width(FLOAT_WIDTH as u64)
                        .height(FLOAT_HEIGHT as u64)
                        .style("minimal".to_string())
                        .border(Border::Single)
                        .noautocmd(true)
                        .focusable(false),
                )
                .await?;
            float_win
                .set(client, "winhl", format!("Normal:{}", DEFAULT_NORMAL_HL))
                .await?;
            float_win.set(client, "diff", false).await?;
        }
        client.lua("vim.cmd('redraw')").await?;
        Ok(())
    }

    /// Pick a window, and return the window ID. If there's only one window, return that window
    /// immediately. Otherwise, display an overlay and ask the user for input.
    #[request]
    async fn pick(&self, client: &mut nvi::Client) -> Result<Window> {
        let current = client.nvim.get_current_win().await?;
        let windows = client
            .nvim
            .tabpage_list_wins(&TabPage::current())
            .await?
            .into_iter()
            .filter(|w| w != &current)
            .collect::<Vec<_>>();

        self.show_hints(client, windows.clone()).await?;

        let c = client
            .lua("return vim.fn.nr2char(vim.fn.getchar())")
            .await?
            .as_str()
            .ok_or(anyhow!("no char"))?
            .chars()
            .next()
            .ok_or(anyhow!("no char"))?;

        let offset = self.keys.find(c).unwrap_or(0);
        if offset >= windows.len() {
            return Err(anyhow!("invalid window"));
        }
        Ok(windows[offset].clone())
    }

    /// Pick a window and jump to it.
    #[request]
    async fn jump(&self, client: &mut nvi::Client) -> Result<()> {
        let window = self.pick(client).await?;
        client.nvim.set_current_win(&window).await?;
        Ok(())
    }

    async fn connected(&self, _client: &mut nvi::Client) -> nvi::error::Result<()> {
        debug!("nvi_win connected");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    nvi::cmd::run(NviWin::new()).await;
}
