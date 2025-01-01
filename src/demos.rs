use nvi::demo::Demos;
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

    d
}
