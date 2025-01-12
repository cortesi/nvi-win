use tokio::time::sleep;

use nvi::demo::Demos;
use nvi::input::feedkeys;
use nvi::lua_exec;

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
        lua_exec!(
            c.clone(),
            "vim.cmd('vsplit'); vim.cmd('split'); vim.cmd('vsplit')"
        )
        .await
        .unwrap();
        for _ in 0..5 {
            let c2 = c.clone();
            tokio::spawn(async move {
                lua_exec!(c2.clone(), "return nvi_win.jump()")
                    .await
                    .unwrap();
            });
            sleep(std::time::Duration::from_secs(1)).await;
            let keys = ['a', 's', 'd'];
            let key = keys[rand::random::<usize>() % 3];
            feedkeys(&c, &key.to_string()).await.unwrap();
        }
        Ok(())
    });

    d
}
