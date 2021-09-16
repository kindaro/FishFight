use macroquad::{
    experimental::collections::storage,
    prelude::*,
    ui::{self, hash, root_ui, widgets},
};

use crate::{gui::GuiResources, input::InputScheme, GameType};

const WINDOW_WIDTH: f32 = 700.;
const WINDOW_HEIGHT: f32 = 400.;

enum MainMenuResult {
    /// Nothing selected yet, keep showing main menu
    None,
    /// Game type selected and is ready to return to main.rs to start a game
    DirectGame(GameType),
    /// Matchmaking should happen to get a GameType, proceeding to a next screen
    MatchmakerGame { stun: bool, input: InputScheme },
}

fn local_game_ui(ui: &mut ui::Ui, players: &mut Vec<InputScheme>) -> MainMenuResult {
    let gui_resources = storage::get_mut::<GuiResources>();

    ui.label(None, "To connect:");
    ui.label(None, "Press Start on gamepad");
    ui.separator();

    ui.label(None, "Or V for keyboard 1");
    ui.label(None, "Or L for keyboard 2");

    ui.separator();
    ui.separator();
    ui.separator();
    ui.separator();

    ui.group(hash!(), vec2(WINDOW_WIDTH / 2. - 50., 70.), |ui| {
        if players.get(0).is_none() {
            ui.label(None, "Player 1: Not connected");
        }
        if let Some(input) = players.get(0) {
            ui.label(None, "Player 1: Connected!");
            ui.label(None, &format!("{:?}", input));
        }
    });
    ui.group(hash!(), vec2(WINDOW_WIDTH / 2. - 50., 70.), |ui| {
        if players.get(1).is_none() {
            ui.label(None, "Player 2: Not connected");
        }
        if let Some(input) = players.get(1) {
            ui.label(None, "Player 2: Connected!");
            ui.label(None, &format!("{:?}", input));
        }
    });
    if players.len() == 2 {
        let btn_a = is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::A);
        let enter = is_key_pressed(KeyCode::Enter);

        if ui.button(None, "Ready! (A) (Enter)") || btn_a || enter {
            return MainMenuResult::DirectGame(GameType::Local(players.clone()));
        }
    }

    MainMenuResult::None
}

struct NetworkUiState {
    stun: bool,
}

async fn connect_through_matchmaker(stun: bool, input_scheme: InputScheme) -> GameType {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();

    let self_addr = if stun {
        let sc = stunclient::StunClient::with_google_stun_server();
        format!("{}", sc.query_external_address(&socket).unwrap())
    } else {
        format!("{}", socket.local_addr().unwrap())
    };
    let mut other_addr = "".to_string();
    let mut res = None;
    let mut socket = Some(socket);

    // skip a frame to skip previous screen Enter press
    next_frame().await;

    loop {
        // to make input field non-editable restoring its content each frame
        let mut self_addr = self_addr.clone();

        let gui_resources = storage::get_mut::<GuiResources>();
        root_ui().push_skin(&gui_resources.skins.login_skin);

        root_ui().window(
            hash!(),
            Vec2::new(
                screen_width() / 2. - WINDOW_WIDTH / 2.,
                screen_height() / 2. - WINDOW_HEIGHT / 2.,
            ),
            Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            |ui| {
                widgets::InputText::new(hash!())
                    .ratio(0.4)
                    .label("Self IP (Copy paste this to Discord)")
                    .ui(ui, &mut self_addr);

                widgets::InputText::new(hash!())
                    .ratio(0.4)
                    .label("Opponent IP")
                    .ui(ui, &mut other_addr);

                let btn_a = is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::A);
                let enter = is_key_pressed(KeyCode::Enter);

                if ui.button(None, "Connect (A) (Enter)") || btn_a || enter {
                    res = Some(GameType::Network {
                        socket: socket.take().unwrap(),
                        other_addr: other_addr.clone(),
                        id: if self_addr > other_addr { 0 } else { 1 },
                        input_scheme,
                    });
                }
            },
        );

        root_ui().pop_skin();

        if let Some(res) = res {
            return res;
        }

        next_frame().await;
    }
}

fn is_gamepad_btn_pressed(gui_resources: &GuiResources, btn: quad_gamepad::GamepadButton) -> bool {
    for ix in 0..quad_gamepad::MAX_DEVICES {
        let state = gui_resources.gamepads.state(ix);
        if state.digital_state[btn as usize] && !state.digital_state_prev[btn as usize] {
            return true;
        }
    }

    return false;
}

fn network_game_ui(
    ui: &mut ui::Ui,
    state: &mut NetworkUiState,
    players: &mut Vec<InputScheme>,
) -> MainMenuResult {
    ui.checkbox(hash!(), "Use STUN server", &mut state.stun);

    if let Some(input) = players.get(0) {
        let gui_resources = storage::get_mut::<GuiResources>();
        let btn_a = is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::A);
        let enter = is_key_pressed(KeyCode::Enter);

        if ui.button(None, "Connect(A) (Enter)") || btn_a || enter {
            return MainMenuResult::MatchmakerGame {
                stun: state.stun,
                input: input.clone(),
            };
        }
    }

    if let Some(input) = players.get(0) {
        ui.label(None, &format!("Input: {:?}", input));
    } else {
        ui.label(None, "To select input scheme:");
        ui.label(None, "Press Start on gamepad");
        ui.separator();

        ui.label(None, "Or V for keyboard 1");
        ui.label(None, "Or L for keyboard 2");
    }

    MainMenuResult::None
}

pub async fn game_type() -> GameType {
    let mut players = vec![];

    let mut network_ui_state = NetworkUiState { stun: true };

    let mut tab = 0;
    loop {
        let mut res = MainMenuResult::None;

        {
            let mut gui_resources = storage::get_mut::<GuiResources>();

            gui_resources.gamepads.update();

            if is_key_pressed(KeyCode::Left)
                || is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::BumperLeft)
                || is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::ThumbLeft)
            {
                tab += 1;
                tab %= 2;
            }
            // for two tabs going left and right is the same thing
            if is_key_pressed(KeyCode::Right)
                || is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::BumperRight)
                || is_gamepad_btn_pressed(&*gui_resources, quad_gamepad::GamepadButton::ThumbRight)
            {
                tab += 1;
                tab %= 2;
            }
            if players.len() < 2 {
                if is_key_pressed(KeyCode::V) {
                    //
                    if !players.contains(&InputScheme::KeyboardLeft) {
                        players.push(InputScheme::KeyboardLeft);
                    }
                }
                if is_key_pressed(KeyCode::L) {
                    //
                    if !players.contains(&InputScheme::KeyboardRight) {
                        players.push(InputScheme::KeyboardRight);
                    }
                }
                for ix in 0..quad_gamepad::MAX_DEVICES {
                    let state = gui_resources.gamepads.state(ix);

                    if state.digital_state[quad_gamepad::GamepadButton::Start as usize] {
                        //
                        if !players.contains(&InputScheme::Gamepad(ix)) {
                            players.push(InputScheme::Gamepad(ix));
                        }
                    }
                }
            }
        }

        {
            let gui_resources = storage::get_mut::<GuiResources>();
            root_ui().push_skin(&gui_resources.skins.login_skin);
        }

        root_ui().window(
            hash!(),
            Vec2::new(
                screen_width() / 2. - WINDOW_WIDTH / 2.,
                screen_height() / 2. - WINDOW_HEIGHT / 2.,
            ),
            Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            |ui| match widgets::Tabbar::new(
                hash!(),
                vec2(WINDOW_WIDTH - 50., 50.),
                &["<< Local game, LT", "Network game, RT >>4"],
            )
            .selected_tab(Some(&mut tab))
            .ui(ui)
            {
                0 => {
                    res = local_game_ui(ui, &mut players);
                }
                1 => {
                    res = network_game_ui(ui, &mut network_ui_state, &mut players);
                }
                _ => unreachable!(),
            },
        );

        root_ui().pop_skin();

        if let MainMenuResult::DirectGame(res) = res {
            return res;
        }
        if let MainMenuResult::MatchmakerGame { stun, input } = res {
            return connect_through_matchmaker(stun, input).await;
        }
        next_frame().await;
    }
}

pub async fn location_select() -> String {
    let mut hovered: i32 = 0;

    let mut old_mouse_position = mouse_position();

    // skip a frame to let Enter be unpressed from the previous screen
    next_frame().await;

    let mut prev_up = false;
    let mut prev_down = false;
    let mut prev_right = false;
    let mut prev_left = false;

    loop {
        let mut gui_resources = storage::get_mut::<GuiResources>();

        gui_resources.gamepads.update();

        let mut up = is_key_pressed(KeyCode::Up);
        let mut down = is_key_pressed(KeyCode::Down);
        let mut right = is_key_pressed(KeyCode::Right);
        let mut left = is_key_pressed(KeyCode::Left);
        let mut start = is_key_pressed(KeyCode::Enter);

        for ix in 0..quad_gamepad::MAX_DEVICES {
            use quad_gamepad::GamepadButton::*;

            let state = gui_resources.gamepads.state(ix);
            if state.status == quad_gamepad::ControllerStatus::Connected {
                up |= !prev_up && state.analog_state[1] < -0.5;
                down |= !prev_down && state.analog_state[1] > 0.5;
                left |= !prev_left && state.analog_state[0] < -0.5;
                right |= !prev_right && state.analog_state[0] > 0.5;
                start |= (state.digital_state[A as usize] && !state.digital_state_prev[A as usize])
                    || (state.digital_state[Start as usize]
                        && !state.digital_state_prev[Start as usize]);

                prev_up = state.analog_state[1] < -0.5;
                prev_down = state.analog_state[1] > 0.5;
                prev_left = state.analog_state[0] < -0.5;
                prev_right = state.analog_state[0] > 0.5;
            }
        }
        clear_background(BLACK);

        let levels_amount = gui_resources.levels.len();

        root_ui().push_skin(&gui_resources.skins.main_menu_skin);

        let rows = (levels_amount + 2) / 3;
        let w = (screen_width() - 120.) / 3. - 50.;
        let h = (screen_height() - 180.) / rows as f32 - 50.;

        {
            if up {
                hovered -= 3;
                let ceiled_levels_amount = levels_amount as i32 + 3 - (levels_amount % 3) as i32;
                if hovered < 0 {
                    hovered = (hovered + ceiled_levels_amount as i32) % ceiled_levels_amount;
                    if hovered >= levels_amount as i32 {
                        hovered -= 3;
                    }
                }
            }

            if down {
                hovered += 3;
                if hovered >= levels_amount as i32 {
                    let row = hovered % 3;
                    hovered = row;
                }
            }
            if left {
                hovered -= 1;
            }
            if right {
                hovered += 1;
            }
            hovered = (hovered + levels_amount as i32) % levels_amount as i32;

            let levels = &mut gui_resources.levels;

            for (n, level) in levels.iter_mut().enumerate() {
                let is_hovered = hovered == n as i32;

                let rect = Rect::new(
                    60. + (n % 3) as f32 * (w + 50.) - level.size * 30.,
                    90. + 25. + (n / 3) as f32 * (h + 50.) - level.size * 30.,
                    w + level.size * 60.,
                    h + level.size * 60.,
                );
                if old_mouse_position != mouse_position() && rect.contains(mouse_position().into())
                {
                    hovered = n as _;
                }

                if is_hovered {
                    level.size = level.size * 0.8 + 1.0 * 0.2;
                } else {
                    level.size = level.size * 0.9 + 0.0;
                }

                if ui::widgets::Button::new(level.preview)
                    .size(rect.size())
                    .position(rect.point())
                    .ui(&mut *root_ui())
                    || start
                {
                    root_ui().pop_skin();
                    let level = &levels[hovered as usize];
                    return level.map.clone();
                }
            }
        }

        root_ui().pop_skin();

        old_mouse_position = mouse_position();

        next_frame().await;
    }
}
