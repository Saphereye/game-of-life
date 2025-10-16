// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashSet;

const CELL_SIZE: f32 = 10.0;
const TICK_SPEED: f32 = 0.07; // seconds per tick
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = f32::MAX;
const ZOOM_SPEED: f32 = 0.1;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<GameOfLife>()
        .init_resource::<SimulationTimer>()
        .init_resource::<SimulationPaused>()
        .init_resource::<DrawMode>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_camera_pan,
                handle_camera_zoom,
                handle_mouse_input,
                handle_keyboard_input,
                update_cursor_preview,
                simulate_game_of_life,
                render_cells,
                update_ui,
            ),
        )
        .run();
}

#[derive(Resource)]
struct GameOfLife {
    alive_cells: HashSet<(i32, i32)>,
}

impl Default for GameOfLife {
    fn default() -> Self {
        Self {
            alive_cells: HashSet::new(),
        }
    }
}

#[derive(Resource)]
struct SimulationTimer {
    timer: Timer,
}

impl Default for SimulationTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(TICK_SPEED, TimerMode::Repeating),
        }
    }
}

#[derive(Resource)]
struct SimulationPaused {
    paused: bool,
}

impl Default for SimulationPaused {
    fn default() -> Self {
        Self { paused: true }
    }
}

#[derive(Resource, PartialEq)]
enum DrawMode {
    Single,   // Draw one cell at a time
    Block3x3, // Draw 3x3 blocks
    Block5x5, // Draw 5x5 blocks
}

impl Default for DrawMode {
    fn default() -> Self {
        Self::Single
    }
}

#[derive(Component)]
struct CellMarker;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct UIText;

#[derive(Component)]
struct CursorPreview;

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));

    // Spawn UI text
    commands.spawn((
        Text::new("Controls:\nSpace: Play/Pause | C: Clear | Mouse Wheel: Zoom\nMiddle Mouse: Pan | 1-3: Draw modes\n\nMode: Single | Paused"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        UIText,
    ));
}

fn handle_camera_pan(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut last_pos: Local<Option<Vec2>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_q.single_mut() else {
        return;
    };

    if mouse_button.pressed(MouseButton::Middle) {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(last) = *last_pos {
                let delta = cursor_pos - last;
                let scale = camera_transform.scale.x;
                camera_transform.translation.x -= delta.x * scale;
                camera_transform.translation.y += delta.y * scale;
            }
            *last_pos = Some(cursor_pos);
        }
    } else {
        *last_pos = None;
    }
}

fn handle_camera_zoom(
    mut scroll_events: EventReader<MouseWheel>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut camera_transform) = camera_q.single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        let zoom_delta = -event.y * ZOOM_SPEED;
        let new_scale = (camera_transform.scale.x + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        camera_transform.scale = Vec3::splat(new_scale);
    }
}

fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut game: ResMut<GameOfLife>,
    draw_mode: Res<DrawMode>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let cell_x = (world_pos.x / CELL_SIZE).floor() as i32;
            let cell_y = (world_pos.y / CELL_SIZE).floor() as i32;

            if mouse_button.pressed(MouseButton::Left) {
                match *draw_mode {
                    DrawMode::Single => {
                        game.alive_cells.insert((cell_x, cell_y));
                    }
                    DrawMode::Block3x3 => {
                        for dx in -1..=1 {
                            for dy in -1..=1 {
                                game.alive_cells.insert((cell_x + dx, cell_y + dy));
                            }
                        }
                    }
                    DrawMode::Block5x5 => {
                        for dx in -2..=2 {
                            for dy in -2..=2 {
                                game.alive_cells.insert((cell_x + dx, cell_y + dy));
                            }
                        }
                    }
                }
            } else if mouse_button.pressed(MouseButton::Right) {
                match *draw_mode {
                    DrawMode::Single => {
                        game.alive_cells.remove(&(cell_x, cell_y));
                    }
                    DrawMode::Block3x3 => {
                        for dx in -1..=1 {
                            for dy in -1..=1 {
                                game.alive_cells.remove(&(cell_x + dx, cell_y + dy));
                            }
                        }
                    }
                    DrawMode::Block5x5 => {
                        for dx in -2..=2 {
                            for dy in -2..=2 {
                                game.alive_cells.remove(&(cell_x + dx, cell_y + dy));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<SimulationPaused>,
    mut game: ResMut<GameOfLife>,
    mut draw_mode: ResMut<DrawMode>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        paused.paused = !paused.paused;
    }

    if keyboard.just_pressed(KeyCode::KeyC) {
        game.alive_cells.clear();
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        *draw_mode = DrawMode::Single;
    }

    if keyboard.just_pressed(KeyCode::Digit2) {
        *draw_mode = DrawMode::Block3x3;
    }

    if keyboard.just_pressed(KeyCode::Digit3) {
        *draw_mode = DrawMode::Block5x5;
    }
}

fn simulate_game_of_life(
    time: Res<Time>,
    mut timer: ResMut<SimulationTimer>,
    mut game: ResMut<GameOfLife>,
    paused: Res<SimulationPaused>,
) {
    if paused.paused {
        return;
    }

    timer.timer.tick(time.delta());

    if timer.timer.just_finished() {
        let mut neighbor_counts: std::collections::HashMap<(i32, i32), u8> =
            std::collections::HashMap::new();

        // Count neighbors for all cells and their neighbors
        for &(x, y) in &game.alive_cells {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let neighbor = (x + dx, y + dy);
                    *neighbor_counts.entry(neighbor).or_insert(0) += 1;
                }
            }
        }

        // Apply Game of Life rules
        let mut new_alive_cells = HashSet::new();

        for (cell, count) in neighbor_counts {
            if count == 3 || (count == 2 && game.alive_cells.contains(&cell)) {
                new_alive_cells.insert(cell);
            }
        }

        game.alive_cells = new_alive_cells;
    }
}

fn update_cursor_preview(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    draw_mode: Res<DrawMode>,
    existing_preview: Query<Entity, With<CursorPreview>>,
) {
    // Clear existing preview
    for entity in &existing_preview {
        commands.entity(entity).despawn();
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let cell_x = (world_pos.x / CELL_SIZE) as i32;
            let cell_y = (world_pos.y / CELL_SIZE) as i32;

            // Determine preview size based on draw mode
            let (size_x, size_y) = match *draw_mode {
                DrawMode::Single => (1, 1),
                DrawMode::Block3x3 => (3, 3),
                DrawMode::Block5x5 => (5, 5),
            };

            let offset = match *draw_mode {
                DrawMode::Single => 0,
                DrawMode::Block3x3 => -1,
                DrawMode::Block5x5 => -2,
            };

            // Draw preview box outline
            let preview_world_x =
                (cell_x + offset) as f32 * CELL_SIZE + (size_x as f32 * CELL_SIZE) / 2.0;
            let preview_world_y =
                (cell_y + offset) as f32 * CELL_SIZE + (size_y as f32 * CELL_SIZE) / 2.0;
            let preview_size_x = size_x as f32 * CELL_SIZE;
            let preview_size_y = size_y as f32 * CELL_SIZE;

            commands.spawn((
                Sprite {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.3),
                    custom_size: Some(Vec2::new(preview_size_x, preview_size_y)),
                    ..default()
                },
                Transform::from_xyz(preview_world_x, preview_world_y, 1.0),
                CursorPreview,
            ));
        }
    }
}

fn render_cells(
    mut commands: Commands,
    game: Res<GameOfLife>,
    existing_cells: Query<Entity, With<CellMarker>>,
) {
    // Only update if the game state changed
    if !game.is_changed() {
        return;
    }

    // Clear existing cell entities
    for entity in &existing_cells {
        commands.entity(entity).despawn();
    }

    // Spawn new cells
    for &(x, y) in &game.alive_cells {
        let world_x = x as f32 * CELL_SIZE;
        let world_y = y as f32 * CELL_SIZE;

        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                ..default()
            },
            Transform::from_xyz(world_x, world_y, 0.0),
            CellMarker,
        ));
    }
}

fn update_ui(
    mut ui_query: Query<&mut Text, With<UIText>>,
    paused: Res<SimulationPaused>,
    draw_mode: Res<DrawMode>,
    game: Res<GameOfLife>,
) {
    if !paused.is_changed() && !draw_mode.is_changed() && !game.is_changed() {
        return;
    }

    let Ok(mut text) = ui_query.single_mut() else {
        return;
    };

    let mode_str = match *draw_mode {
        DrawMode::Single => "Single",
        DrawMode::Block3x3 => "3x3 Block",
        DrawMode::Block5x5 => "5x5 Block",
    };

    let status = if paused.paused { "Paused" } else { "Running" };
    let cell_count = game.alive_cells.len();

    text.0 = format!(
        "Controls:\nSpace: Play/Pause | C: Clear | Mouse Wheel: Zoom\nMiddle Mouse: Pan | 1-3: Draw modes\n\nMode: {} | {} | Cells: {}",
        mode_str, status, cell_count
    );
}
