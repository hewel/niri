use super::*;

fn setup() -> Layout<TestWindow> {
    setup_with_gap_and_scale(Options::default().layout.gaps, 1.)
}

fn setup_with_gap_and_scale(gap: f64, scale: f64) -> Layout<TestWindow> {
    let mut options = Options::default();
    options.layout.tab_indicator.show_title = true;
    options.layout.gaps = gap;
    let mut layout = check_ops_with_options(
        options,
        [
            Op::AddScaledOutput {
                id: 1,
                scale,
                layout_config: None,
            },
            Op::AddWindow {
                params: TestWindowParams::new(1),
            },
            Op::AddWindow {
                params: TestWindowParams::new(2),
            },
            Op::FocusColumnLeft,
            Op::ConsumeWindowIntoColumn,
            Op::SetColumnDisplay(ColumnDisplay::Tabbed),
            Op::AddWindow {
                params: TestWindowParams::new(3),
            },
            Op::ConsumeOrExpelWindowLeft { id: Some(3) },
        ],
    );
    settle(&mut layout);
    layout
}

fn settle(layout: &mut Layout<TestWindow>) {
    for id in 1..=4 {
        Op::Communicate(id).apply(layout);
    }
    Op::CompleteAnimations.apply(layout);
    layout.verify_invariants();
}

fn columns(layout: &Layout<TestWindow>) -> Vec<Vec<usize>> {
    layout
        .active_workspace()
        .unwrap()
        .scrolling()
        .columns()
        .map(|col| col.tiles().map(|(tile, _)| *tile.window().id()).collect())
        .collect()
}

// Sample the real rail hit surface, including physical rounding, along its vertical axis.
fn visible_tab_point(
    layout: &Layout<TestWindow>,
    id: usize,
    trailing: bool,
) -> Option<Point<f64, Logical>> {
    let output = layout.outputs().next().unwrap();
    let size = output.current_mode().unwrap().size;
    let scale = output.current_scale().fractional_scale();
    for x in (0..size.w).step_by(2) {
        let mut first = None;
        let mut last = None;
        for y in (0..size.h).step_by(2) {
            let point = Point::from(((x as f64 + 0.5) / scale, (y as f64 + 0.5) / scale));
            if layout
                .window_under(output, point)
                .is_some_and(|(w, hit)| *w.id() == id && matches!(hit, HitType::Tab))
            {
                first.get_or_insert(point);
                last = Some(point);
            }
        }
        if let Some(first) = first {
            return Some(if trailing { last.unwrap() } else { first });
        }
    }
    None
}

fn tab_point(layout: &Layout<TestWindow>, id: usize, trailing: bool) -> Point<f64, Logical> {
    visible_tab_point(layout, id, trailing)
        .unwrap_or_else(|| panic!("no visible tab body for window {id}"))
}

fn drag_to_tab(
    layout: &mut Layout<TestWindow>,
    output: &Output,
    dragged: usize,
    destination: usize,
    trailing: bool,
) {
    let target = tab_point(layout, destination, trailing);
    layout.interactive_move_update(&dragged, (0., 0.).into(), output.clone(), target);
}

fn begin_drag(layout: &mut Layout<TestWindow>, id: usize) -> Output {
    layout.set_tab_hover(Some(&id));
    let start = tab_point(layout, id, false);
    let output = layout.outputs().next().unwrap().clone();
    layout.activate_window(&id);
    assert!(layout.interactive_move_begin(id, &output, start, true));
    assert!(layout.interactive_move_update(
        &id,
        (0., 12.).into(),
        output.clone(),
        start + Point::from((0., 12.))
    ));
    assert!(matches!(
        layout.interactive_move,
        Some(InteractiveMoveState::Moving(_))
    ));
    settle(layout);
    output
}

#[test]
fn tab_scroll_uses_active_tab_not_hovered_tab_and_clamps() {
    let mut layout = setup();
    assert_eq!(columns(&layout), vec![vec![1, 2, 3]]);
    layout.activate_window(&2);
    assert!(layout.scroll_tab(&1, 1));
    assert_eq!(
        *layout
            .active_workspace()
            .unwrap()
            .active_window()
            .unwrap()
            .id(),
        3
    );
    assert!(!layout.scroll_tab(&1, 1));
    assert!(layout.scroll_tab(&3, i32::MIN));
    assert_eq!(
        *layout
            .active_workspace()
            .unwrap()
            .active_window()
            .unwrap()
            .id(),
        1
    );
    assert!(!layout.scroll_tab(&1, -1));
    Op::CloseWindow(3).apply(&mut layout);
    assert!(!layout.scroll_tab(&3, 1));
    layout.verify_invariants();
}

#[test]
fn tab_drop_target_prevents_edge_scroll_until_pointer_leaves() {
    let mut layout = setup();
    let output = begin_drag(&mut layout, 3);
    drag_to_tab(&mut layout, &output, 3, 1, false);
    let before = client_area(&layout, 1);

    let scroll_for_two_seconds = |layout: &mut Layout<TestWindow>| {
        for _ in 0..3 {
            let now = layout.clock.now_unadjusted() + Duration::from_secs(1);
            layout.clock.set_unadjusted(now);
            layout
                .active_workspace_mut()
                .unwrap()
                .scrolling_mut()
                .dnd_scroll_gesture_scroll(-1.);
        }
    };
    scroll_for_two_seconds(&mut layout);
    assert_eq!(client_area(&layout, 1), before);

    // The same screen edge below the rail must still pan the workspace.
    let outside = Point::from((tab_point(&layout, 1, false).x, 200.));
    layout.interactive_move_update(&3, (0., 0.).into(), output, outside);
    scroll_for_two_seconds(&mut layout);
    assert_ne!(client_area(&layout, 1).loc, before.loc);
    layout.interactive_move_end(&3);
}

#[test]
fn tab_drag_reorders_at_both_column_ends() {
    let mut layout = setup();
    let output = begin_drag(&mut layout, 3);
    drag_to_tab(&mut layout, &output, 3, 1, false);
    layout.interactive_move_end(&3);
    settle(&mut layout);
    assert_eq!(columns(&layout), vec![vec![3, 1, 2]]);

    let output = begin_drag(&mut layout, 3);
    drag_to_tab(&mut layout, &output, 3, 2, true);
    layout.interactive_move_end(&3);
    settle(&mut layout);
    assert_eq!(columns(&layout), vec![vec![1, 2, 3]]);
}

#[test]
fn tab_drag_off_rail_splits_instead_of_consuming_near_tile_edge() {
    let mut layout = setup();
    let output = begin_drag(&mut layout, 2);
    drag_to_tab(&mut layout, &output, 2, 1, false);
    // Over the source column's content, near its upper edge: ordinary dragging could merge here.
    let target = client_area(&layout, 1).loc + Point::from((10., 10.));
    layout.interactive_move_update(&2, (0., 0.).into(), output, target);
    layout.interactive_move_end(&2);
    settle(&mut layout);
    let mut actual = columns(&layout);
    actual.sort();
    assert_eq!(actual, vec![vec![1, 3], vec![2]]);
}

#[test]
fn tab_drag_merges_into_other_column_at_selected_boundary() {
    let mut layout = setup();
    Op::AddWindow {
        params: TestWindowParams::new(4),
    }
    .apply(&mut layout);
    layout.set_column_display(ColumnDisplay::Tabbed);
    settle(&mut layout);
    assert!(visible_tab_point(&layout, 4, false).is_none());
    let output = begin_drag(&mut layout, 2);
    assert!(visible_tab_point(&layout, 4, false).is_some());
    drag_to_tab(&mut layout, &output, 2, 4, true);
    layout.interactive_move_end(&2);
    settle(&mut layout);
    assert_eq!(columns(&layout), vec![vec![1, 3], vec![4, 2]]);
}

#[test]
fn tab_drag_recomputes_destination_after_target_closes() {
    let mut layout = setup();
    Op::AddWindow {
        params: TestWindowParams::new(4),
    }
    .apply(&mut layout);
    layout.set_column_display(ColumnDisplay::Tabbed);
    settle(&mut layout);
    let output = begin_drag(&mut layout, 2);
    drag_to_tab(&mut layout, &output, 2, 4, false);
    Op::CloseWindow(4).apply(&mut layout);
    layout.interactive_move_end(&2);
    settle(&mut layout);
    let mut actual = columns(&layout);
    actual.sort();
    assert_eq!(actual, vec![vec![1, 3], vec![2]]);
}

fn client_area(layout: &Layout<TestWindow>, id: usize) -> Rectangle<f64, Logical> {
    let (tile, pos, _) = layout
        .active_workspace()
        .unwrap()
        .tiles_with_render_positions()
        .find(|(tile, _, _)| *tile.window().id() == id)
        .unwrap();
    Rectangle::new(pos + tile.window_loc(), tile.window_size())
}

fn window_sizes(layout: &Layout<TestWindow>) -> Vec<(usize, Size<i32, Logical>)> {
    let mut sizes = layout
        .windows()
        .map(|(_, window)| (*window.id(), window.size()))
        .collect::<Vec<_>>();
    sizes.sort_by_key(|(id, _)| *id);
    sizes
}

#[test]
fn tab_rail_never_intercepts_client_content() {
    for gap in [Options::default().layout.gaps, 0., -8.] {
        for scale in [1., 1.25, 1.5] {
            let mut layout = setup_with_gap_and_scale(gap, scale);
            layout.activate_window(&2);
            settle(&mut layout);
            let output = layout.outputs().next().unwrap().clone();
            let output_size = output.current_mode().unwrap().size;
            let area = client_area(&layout, 2);
            let initial_sizes = window_sizes(&layout);
            let rail_points = (1..=3)
                .map(|id| (tab_point(&layout, id, false), tab_point(&layout, id, true)))
                .collect::<Vec<_>>();
            for (first, last) in &rail_points {
                assert!(first.x < area.loc.x && last.x < area.loc.x);
                assert!(first.y < last.y);
            }
            assert!(rail_points.windows(2).all(|pair| pair[0].1.y < pair[1].0.y));

            assert!(layout.set_tab_hover(Some(&2)));
            settle(&mut layout);
            assert_eq!(window_sizes(&layout), initial_sizes);
            for (id, points) in (1..=3).zip(&rail_points) {
                assert_eq!(tab_point(&layout, id, false), points.0);
                assert_eq!(tab_point(&layout, id, true), points.1);
            }

            // Include the leftmost physical-pixel center, where a rounded gutter could leak.
            // TestWindow has no client input region; the tile must remain activatable, not a tab.
            for py in 0..(area.size.h * scale).floor() as i32 {
                for px in 0..(area.size.w * scale).floor() as i32 {
                    let point = area.loc
                        + Point::from(((px as f64 + 0.5) / scale, (py as f64 + 0.5) / scale));
                    if point.x < 0.
                        || point.y < 0.
                        || point.x >= output_size.w as f64 / scale
                        || point.y >= output_size.h as f64 / scale
                    {
                        continue;
                    }
                    let (window, hit) = layout.window_under(&output, point).unwrap();
                    assert_eq!(*window.id(), 2);
                    assert!(
                        matches!(hit, HitType::Activate),
                        "gap={gap}, scale={scale}, point={point:?}"
                    );
                }
            }
            assert!(layout.set_tab_hover(None));
            settle(&mut layout);
            assert_eq!(window_sizes(&layout), initial_sizes);
        }
    }
}

#[test]
fn tab_rail_reuses_gap_and_reserves_only_missing_width() {
    // Use column-local offsets so automatic viewport scrolling cannot affect the comparison.
    let client_offset = |layout: &Layout<TestWindow>| {
        let col = layout
            .active_workspace()
            .unwrap()
            .scrolling()
            .columns()
            .next()
            .unwrap();
        let (tile, pos) = col.tiles().next().unwrap();
        pos.x + tile.window_loc().x
    };
    let default_gap = Options::default().layout.gaps;
    for gap in [default_gap, 0., -8.] {
        let mut layout = setup_with_gap_and_scale(gap, 1.);
        let effective_gap = (*layout.options).clone().adjusted_for_scale(1.).layout.gaps;
        Op::SetColumnWidth(SizeChange::SetFixed(560)).apply(&mut layout);
        settle(&mut layout);
        let with_rail = client_area(&layout, 2);
        let with_rail_offset = client_offset(&layout);
        let mut options = (*layout.options).clone();
        options.layout.tab_indicator.off = true;
        layout.update_options(options);
        settle(&mut layout);
        let without_rail = client_area(&layout, 2);
        assert_eq!(with_rail.size.h, without_rail.size.h);
        assert_eq!(with_rail.size.w, without_rail.size.w);
        let reserved = with_rail_offset - client_offset(&layout);
        if gap == default_gap {
            assert_eq!(
                reserved, 0.,
                "the existing default gap must accommodate the rail"
            );
        } else {
            assert!(
                reserved > 0.,
                "a missing gutter must reserve additional width"
            );
            // Use the effective gap after the layout's physical-pixel normalization.
            assert_eq!(reserved + effective_gap, default_gap);
        }

        // Proportional columns keep their footprint instead of their client width.
        let mut layout = setup_with_gap_and_scale(gap, 1.);
        Op::SetColumnWidth(SizeChange::SetProportion(50.)).apply(&mut layout);
        settle(&mut layout);
        let with_rail = client_area(&layout, 2);
        let with_rail_offset = client_offset(&layout);
        let mut options = (*layout.options).clone();
        options.layout.tab_indicator.off = true;
        layout.update_options(options);
        settle(&mut layout);
        let without_rail = client_area(&layout, 2);
        assert_eq!(with_rail.size.h, without_rail.size.h);
        assert_eq!(without_rail.size.w - with_rail.size.w, reserved);
        assert_eq!(
            with_rail_offset + with_rail.size.w,
            client_offset(&layout) + without_rail.size.w
        );
    }
}

#[test]
fn single_tab_drop_affordance_disappears_after_drag_leaves_and_ends() {
    for gap in [Options::default().layout.gaps, 0.] {
        let mut layout = setup_with_gap_and_scale(gap, 1.);
        Op::AddWindow {
            params: TestWindowParams::new(4),
        }
        .apply(&mut layout);
        layout.set_column_display(ColumnDisplay::Tabbed);
        settle(&mut layout);
        assert!(visible_tab_point(&layout, 4, false).is_none());
        let initial_size = client_area(&layout, 4).size;

        let output = begin_drag(&mut layout, 2);
        let target = tab_point(&layout, 4, false);
        assert_eq!(client_area(&layout, 4).size, initial_size);
        layout.interactive_move_update(&2, (0., 0.).into(), output.clone(), target);
        assert!(visible_tab_point(&layout, 4, false).is_some());

        // Leave the single-window destination and return to the source before dropping.
        drag_to_tab(&mut layout, &output, 2, 1, false);
        assert!(visible_tab_point(&layout, 4, false).is_some());
        layout.interactive_move_end(&2);
        settle(&mut layout);
        assert!(visible_tab_point(&layout, 4, false).is_none());
        assert_eq!(client_area(&layout, 4).size, initial_size);
        assert_eq!(columns(&layout), vec![vec![2, 1, 3], vec![4]]);
    }
}
