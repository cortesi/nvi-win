use std::time::Duration;

use nvi::{demo::Demos, input::feedkeys, lua_exec};
use tokio::time::sleep;

/// Register the demos for the plugin.
pub fn demos() -> Demos {
    let mut d = Demos::new();
    d.add("startup", |c| async move {
        lua_exec!(c, "vim.cmd('vsplit'); vim.cmd('split')")
            .await
            .unwrap();
        lua_exec!(c, "return nvi_win.jump()").await.unwrap();
        Ok(())
    });
    d.add("cycle", |c| async move {
        lua_exec!(c, "vim.cmd('vsplit'); vim.cmd('split'); vim.cmd('vsplit')")
            .await
            .unwrap();
        for _ in 0..5 {
            let c2 = c.clone();
            tokio::spawn(async move {
                lua_exec!(c2.clone(), "return nvi_win.jump()")
                    .await
                    .unwrap();
            });
            sleep(Duration::from_secs(1)).await;
            let keys = ['a', 's', 'd'];
            let key = keys[rand::random::<usize>() % 3];
            feedkeys(&c, &key.to_string()).await.unwrap();
        }
        Ok(())
    });
    d.add("nextprev", |c| async move {
        lua_exec!(
            c.clone(),
            "vim.cmd('vsplit'); vim.cmd('split'); vim.cmd('vsplit')"
        )
        .await
        .unwrap();

        for _ in 0..7 {
            sleep(Duration::from_secs(1)).await;
            lua_exec!(c, "return nvi_win.next()").await.unwrap();
        }

        for _ in 0..7 {
            sleep(Duration::from_secs(1)).await;
            lua_exec!(c, "return nvi_win.prev()").await.unwrap();
        }

        Ok(())
    });

    d
}
