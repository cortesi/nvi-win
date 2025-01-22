use crate::{find_dir, Dir, NviWin};
use futures_util::future::FutureExt;
use nvi::test::NviTest;
use nvi::{lua, lua_exec};

#[tokio::test]
async fn find_dir_basic() {
    let geoms = vec![
        // Ensure windows touch edges exactly
        (0, 0, 10, 10),   // 0
        (9, 0, 10, 10),   // 1 (1px horizontal overlap)
        (0, 10, 10, 10),  // 2 (adjacent bottom edge)
        (10, 10, 10, 10), // 3 (exact right edge adjacency to 2)
    ];

    // Window layout:
    // 0 (0,0) → 1 (10,0)
    // ↓
    // 2 (0,10) → 3 (20,10)
    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        Some(1),
        "Must detect 1px horizontal overlap"
    );
    assert_eq!(
        find_dir(Dir::Down, 0, &geoms),
        Some(2),
        "Must detect 1px vertical overlap"
    );
    assert_eq!(find_dir(Dir::Left, 1, &geoms), Some(0));
    assert_eq!(find_dir(Dir::Down, 0, &geoms), Some(2));
    assert_eq!(find_dir(Dir::Right, 2, &geoms), Some(3));
    assert_eq!(find_dir(Dir::Left, 3, &geoms), Some(2));
    assert_eq!(find_dir(Dir::Up, 2, &geoms), Some(0)); // Fixed: Up from 2 should find 0
}

#[tokio::test]
async fn find_dir_overlapping() {
    let geoms = vec![
        (0, 0, 10, 10),  // 0
        (9, 0, 10, 10),  // 1 (1px horizontal overlap)
        (0, 9, 10, 10),  // 2 (overlapping 1px vertically)
        (19, 0, 10, 10), // 3 (1px overlap with window 1)
    ];

    // Verify overlapping takes priority over gap candidates
    assert_eq!(find_dir(Dir::Right, 0, &geoms), Some(1));
    assert_eq!(find_dir(Dir::Down, 0, &geoms), Some(2));
    assert_eq!(
        find_dir(Dir::Right, 1, &geoms),
        Some(3),
        "Should detect 1px overlap"
    );
}

#[tokio::test]
async fn find_dir_edge_cases() {
    let geoms = vec![
        (0, 0, 10, 10),
        (9, 0, 10, 10),  // 1px horizontal overlap
        (0, 9, 10, 10),  // overlaps 1px vertically
        (19, 0, 10, 10), // 3 (1px overlap with window 1)
    ];

    // Verify horizontal overlap detection
    // Horizontal overlap takes priority over gap candidates
    assert_eq!(find_dir(Dir::Right, 0, &geoms), Some(1));
    // Vertical overlap takes priority
    assert_eq!(find_dir(Dir::Down, 0, &geoms), Some(2));
    // Verify gap handling
    assert_eq!(
        find_dir(Dir::Right, 1, &geoms),
        Some(3),
        "Must detect adjacent right window"
    );
}

#[tokio::test]
async fn find_dir_multiple_candidates_same_direction() {
    let geoms = vec![
        (0, 0, 10, 10),  // 0
        (15, 0, 10, 10), // 1 (distance 5)
        (11, 0, 10, 10), // 2 (distance 1)
        (12, 0, 10, 10), // 3 (distance 2)
    ];

    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        Some(2),
        "Closest window should be selected from multiple candidates"
    );
}

#[tokio::test]
async fn find_dir_partial_overlap() {
    let geoms = vec![
        (0, 0, 10, 10), // 0
        (10, 5, 10, 5), // 1 (partial y overlap)
    ];

    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        Some(1),
        "Should consider windows with partial y-axis overlap"
    );
}

#[tokio::test]
async fn find_dir_no_perpendicular_overlap() {
    let geoms = vec![
        (0, 0, 10, 10),   // 0
        (10, 15, 10, 10), // 1 (no y overlap)
    ];

    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        None,
        "Should exclude windows without perpendicular axis overlap"
    );
}

#[tokio::test]
async fn find_dir_diagonal_no_overlap() {
    let geoms = vec![
        (0, 0, 10, 10),   // 0
        (20, 20, 10, 10), // 1 (diagonal placement)
    ];

    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        None,
        "Should ignore diagonally placed windows without axis overlap"
    );
    assert_eq!(
        find_dir(Dir::Down, 0, &geoms),
        None,
        "Should ignore diagonally placed windows without axis overlap"
    );
}

#[tokio::test]
async fn find_dir_no_windows_in_direction() {
    let geoms = vec![
        (0, 0, 10, 10), // 0 (only window)
    ];

    assert_eq!(
        find_dir(Dir::Right, 0, &geoms),
        None,
        "Should return None when no windows exist in direction"
    );
}

#[tokio::test]
async fn startup() {
    let nvit = NviTest::builder()
        .show_logs()
        .log_level(tracing::Level::DEBUG)
        .with_plugin(NviWin::new())
        .run()
        .await
        .unwrap();

    lua_exec!(nvit.client, "vim.cmd('vsplit'); vim.cmd('split')")
        .await
        .unwrap();

    let current = nvit.client.nvim.get_current_win().await.unwrap();
    let result: u64 = nvit
        .concurrent(
            |c| async move { lua!(c, "return nvi_win.pick()").await }.boxed(),
            |c| async move { c.nvim.feedkeys("a", "n", false).await }.boxed(),
        )
        .await
        .unwrap();

    assert!(result != current.into());
    nvit.finish().await.unwrap();
}

#[tokio::test]
async fn test_directional() {
    let nvit = NviTest::builder()
        .show_logs()
        .log_level(tracing::Level::DEBUG)
        .with_plugin(NviWin::new())
        .run()
        .await
        .unwrap();

    // Create a horizontal split
    lua_exec!(nvit.client, "vim.cmd('vsplit')").await.unwrap();

    let start = nvit.client.nvim.get_current_win().await.unwrap();

    // Move right
    lua_exec!(nvit.client, "return nvi_win.right()")
        .await
        .unwrap();
    let after_right = nvit.client.nvim.get_current_win().await.unwrap();
    assert!(start != after_right, "right() should change window");

    // Move left should return to start
    lua_exec!(nvit.client, "return nvi_win.left()")
        .await
        .unwrap();
    let after_left = nvit.client.nvim.get_current_win().await.unwrap();
    assert_eq!(start, after_left, "left() should return to original window");

    nvit.finish().await.unwrap();
}

#[tokio::test]
async fn pick() {
    let nvit = NviTest::builder()
        .show_logs()
        .log_level(tracing::Level::DEBUG)
        .with_plugin(NviWin::new())
        .run()
        .await
        .unwrap();

    lua_exec!(nvit.client, "vim.cmd('vsplit'); vim.cmd('split')")
        .await
        .unwrap();

    let before = nvit.client.nvim.get_current_win().await.unwrap();
    nvit.concurrent(
        |c| async move { lua_exec!(c, "return nvi_win.jump()").await }.boxed(),
        |c| async move { c.nvim.feedkeys("a", "n", false).await }.boxed(),
    )
    .await
    .unwrap();
    let after = nvit.client.nvim.get_current_win().await.unwrap();

    assert!(before != after);
    nvit.finish().await.unwrap();
}
