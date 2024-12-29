use anyhow::{anyhow, Result};
use nvi::nvi_macros::*;
use nvi::types::{TabPage, Window};
use tracing::debug;

#[cfg(test)]
mod tests;

const DEFAULT_KEYS: &str = "asdfghjklqwertyuiopzxcvbnm";

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
