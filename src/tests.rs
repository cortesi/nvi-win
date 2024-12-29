use futures_util::future::FutureExt;
use nvi::test::NviTest;

use crate::NviWin;

#[tokio::test]
async fn startup() {
    let nvit = NviTest::builder()
        .show_logs()
        .log_level(tracing::Level::DEBUG)
        .run(NviWin::new())
        .await
        .unwrap();

    nvit.client
        .lua("vim.cmd('vsplit'); vim.cmd('split')")
        .await
        .unwrap();

    let current = nvit.client.nvim.get_current_win().await.unwrap();

    nvit.await_log("nvi_win connected").await.unwrap();

    let result = nvit
        .concurrent(
            |c| async move { c.lua("return nvi_win.pick()").await }.boxed(),
            |c| async move { c.nvim.feedkeys("a", "n", false).await }.boxed(),
        )
        .await
        .unwrap();

    assert!(result.as_u64().unwrap() != current.into());
    nvit.finish().await.unwrap();
}

#[tokio::test]
async fn pick() {
    let nvit = NviTest::builder()
        .show_logs()
        .log_level(tracing::Level::DEBUG)
        .run(NviWin::new())
        .await
        .unwrap();

    nvit.client
        .lua("vim.cmd('vsplit'); vim.cmd('split')")
        .await
        .unwrap();

    nvit.await_log("nvi_win connected").await.unwrap();

    let before = nvit.client.nvim.get_current_win().await.unwrap();
    nvit.concurrent(
        |c| async move { c.lua("return nvi_win.jump()").await }.boxed(),
        |c| async move { c.nvim.feedkeys("a", "n", false).await }.boxed(),
    )
    .await
    .unwrap();
    let after = nvit.client.nvim.get_current_win().await.unwrap();

    assert!(before != after);
    nvit.finish().await.unwrap();
}

