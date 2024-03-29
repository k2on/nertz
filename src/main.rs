use colorgrad::{Color, CustomGradient, Gradient};
use std::collections::HashMap;

use gloo::{
    console,
    storage::{LocalStorage, Storage},
};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use web_sys::HtmlInputElement;
use yew::{html::Scope, prelude::*};

#[derive(Debug, Serialize, Deserialize)]
struct State {
    players: Vec<Player>,
    scores: Vec<Vec<Score>>,
    is_in_progress: bool,
    first_to: u8,
    negative_size: u8,
    deck_size: u8,
}

type Player = String;
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Score {
    val: Option<i8>,
    is_editing: bool,
}

pub struct App {
    state: State,
    refs: HashMap<String, NodeRef>,
    leaderboard: Leaderboard,
    gradient: Gradient,
}

type Leaderboard = Vec<usize>;

impl State {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            scores: Vec::new(),
            is_in_progress: false,
            first_to: 100, // the game ends when a player hits this number
            negative_size: 13,
            deck_size: 52,
        }
    }

    fn next_round(&mut self) {
        let mut round: Vec<Score> = std::iter::repeat(Score {
            val: None,
            is_editing: false,
        })
        .take(self.players.len())
        .collect();
        round[0].is_editing = true;
        self.scores.insert(0, round);
    }

    fn player_sum(&self, idx: usize) -> i8 {
        self.scores
            .iter()
            .filter_map(|round| {
                if let Some(val) = round[idx].val {
                    Some(val)
                } else {
                    None
                }
            })
            .sum()
    }

    fn is_game_over(&self) -> bool {
        if self
            .scores
            .clone()
            .iter_mut()
            .any(|round| round.iter_mut().any(|score| score.val.is_none()))
        {
            return false;
        }

        let mut scores = self
            .players
            .iter()
            .enumerate()
            .map(|(idx, _)| self.player_sum(idx));

        if scores.len() == 0 {
            return false;
        }

        // console::log!(to_string(&format!("{:?}", scores.clone().collect::<Vec<i8>>())).unwrap());
        let max = &scores.clone().max().unwrap();
        let h = scores
            .clone()
            .filter(|score| score.eq(max))
            .collect::<Vec<i8>>();

        // console::log!(to_string(&format!("{:?}", h)).unwrap());

        let game_has_reached_max_score = scores.any(|score| score >= self.first_to as i8);

        // console::log!(to_string(&format!("{:?}", h.iter().count())).unwrap());

        let no_tie = h.iter().count().eq(&1);

        // console::log!(to_string(&format!("{} {}", game_has_reached_max_score, no_tie)).unwrap());

        game_has_reached_max_score && no_tie
    }

    fn get_leader_board(&self) -> Leaderboard {
        let mut scores = self
            .players
            .iter()
            .enumerate()
            .map(|(idx, _)| (idx, self.player_sum(idx)))
            .collect::<Vec<(usize, i8)>>();
        scores.sort_by(|(_, a), (_, b)| b.cmp(a));
        scores.iter().map(|(player_idx, _)| *player_idx).collect()
    }
}

const KEY: &str = "yew.nertzpro.self";

pub enum AppMsg {
    ScoreEnter(usize, usize, i8),
    ScoreEdit(usize, usize),
    GameNew,
    GameStart,
    PlayerAdd(String),
    PlayerRemove(usize),
}

impl App {
    pub fn get_focused(&self) -> String {
        let (round, player) = self
            .state
            .scores
            .iter()
            .enumerate()
            .find_map(|(round_idx, round)| {
                round
                    .iter()
                    .enumerate()
                    .position(|(_, score)| score.is_editing)
                    .map(|player_idx| (round_idx, player_idx))
            })
            .unwrap_or((0, 0)); // You can use any default value here, (-1, -1) is just an example

        format!("{}_{}", round, player)
    }

    fn next_round(&mut self) {
        self.state.next_round();
        for (round_idx, round) in self.state.scores.iter().enumerate() {
            for (player_idx, _) in round.iter().enumerate() {
                let key = format!("{}_{}", round_idx, player_idx);
                if let None = self.refs.get(&key) {
                    self.refs.insert(key, NodeRef::default());
                }
            }
        }
    }

    fn view_input(&self, link: &Scope<Self>) -> Html {
        let onkeypress = link.batch_callback(|e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                let value = input.value();
                input.set_value("");
                Some(AppMsg::PlayerAdd(value))
            } else {
                None
            }
        });
        html! {
            <input
                class="new-player"
                placeholder="Add player"
                {onkeypress}
            />
        }
    }

    fn view_player(&self, idx: usize, player: &Player, link: &Scope<App>) -> Html {
        let onclick = link.callback(move |_| AppMsg::PlayerRemove(idx));
        html! {
            <li>
                <label>{player}</label>
                <button class="remove" {onclick}>{"x"}</button>
            </li>
        }
    }

    fn view_player_sum(&self, idx: usize) -> Html {
        let sum = self.state.player_sum(idx);
        html! {
            <div>
                {sum}
            </div>
        }
    }

    fn find_unique_prefix(&self, idx: usize) -> String {
        let current_name = self.state.players.get(idx).unwrap();
        let mut prefix_len = 1;

        while prefix_len <= current_name.len() {
            let current_prefix = &current_name[..prefix_len];
            let mut is_unique = true;

            for (other_idx, other_name) in self.state.players.iter().enumerate() {
                if other_idx != idx {
                    let other_prefix = &other_name[..std::cmp::min(prefix_len, other_name.len())];
                    if current_prefix.to_lowercase() == other_prefix.to_lowercase() {
                        is_unique = false;
                        break;
                    }
                }
            }

            if is_unique {
                return current_prefix.to_string();
            }

            prefix_len += 1;
        }

        current_name.to_string()
    }

    fn get_next_empty(&mut self) -> Option<&mut Score> {
        self.state
            .scores
            .iter_mut()
            .rev()
            .find_map(|round| round.iter_mut().find(|score| score.val.is_none()))
    }
}

fn make_refs(state: &State) -> HashMap<String, NodeRef> {
    let mut refs = HashMap::new();
    for (round_idx, round) in state.scores.iter().enumerate() {
        for (player_idx, _) in round.iter().enumerate() {
            let key = format!("{}_{}", round_idx, player_idx);
            refs.insert(key, NodeRef::default());
        }
    }
    refs
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let state = LocalStorage::get(KEY).unwrap_or_else(|_| State::new());

        let refs = make_refs(&state);
        let leaderboard = state.get_leader_board();

        let red = Color::from_rgba8(255, 0, 0, 255);
        let green = Color::from_rgba8(0, 255, 0, 255);
        let blue = Color::from_rgba8(0, 212, 255, 255);

        let gradient = CustomGradient::new()
            .colors(&[red, green, blue])
            .build()
            .unwrap();

        Self {
            state,
            refs,
            leaderboard,
            gradient,
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if self.state.is_in_progress {
            let node_ref = self.refs.get(&self.get_focused()).unwrap();

            if let Some(input) = node_ref.cast::<HtmlInputElement>() {
                input.focus().unwrap();
            }
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::ScoreEnter(round, player, score) => {
                self.state.scores[round][player] = Score {
                    val: Some(score),
                    is_editing: false,
                };
                if let Some(score) = self.get_next_empty() {
                    score.is_editing = true;
                } else if !self.state.is_game_over() {
                    self.next_round();
                }
            }
            AppMsg::ScoreEdit(round_idx_edit, player_idx_edit) => self
                .state
                .scores
                .iter_mut()
                .enumerate()
                .for_each(|(round_idx, round)| {
                    round
                        .iter_mut()
                        .enumerate()
                        .for_each(|(player_idx, score)| {
                            let should_edit =
                                round_idx == round_idx_edit && player_idx == player_idx_edit;
                            score.is_editing = should_edit;
                        })
                }),
            AppMsg::GameNew => {
                let players = self.state.players.clone();
                self.state = State::new();
                self.state.players = players;
            }
            AppMsg::GameStart => {
                self.state.is_in_progress = true;
                self.next_round();
            }
            AppMsg::PlayerAdd(name) => self.state.players.push(name),
            AppMsg::PlayerRemove(idx) => {
                self.state.players.remove(idx);
            }
        }
        self.leaderboard = self.state.get_leader_board();

        LocalStorage::set(KEY, &self.state).expect("failed to set");
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        console::log!(to_string(&format!("{:?}", self.state)).unwrap());
        let is_game_over = self.state.is_game_over();

        html! {
            <div class="container">
                <img id="logo" src="static/logo.png" alt="NERTS.PRO"/>
                {if self.state.is_in_progress {
                    html! {
                        <div>
                <table class="scores">

                <tr>
                    { for self.state.players.iter().enumerate().map(|(idx, player)| html! {
                        <td>{if is_game_over {
                            let (place_idx, _) = self.leaderboard.iter().enumerate().find(|(_, &p)| idx.eq(&p)).unwrap();

                            let metal = match place_idx {
                                0 => Some("gold"),
                                1 => Some("silver"),
                                2 => Some("bronze"),
                                _ => None
                            };
                            if let Some(metal) = metal {
                                let url = format!("static/{}.png", metal);
                                html! {
                                    <img class="metal" src={url}/>
                                }
                            } else {
                                html! {}
                            }
                        } else {
                            html! {}
                        }}
                        {self.find_unique_prefix(idx)}
                        {if is_game_over { html! { <><br/><br/></> } } else { html! {} }}
                        {self.view_player_sum(idx)}</td>
                    }) }
                </tr>

                { for self.state.scores.iter().enumerate().map(|(round_idx, round)| html! {
                    <tr>
                    { for round.iter().enumerate().map(|(player_idx, score)| {

                        let key = format!("{}_{}", round_idx, player_idx);
                        let node_ref = self.refs.get(&key).unwrap();

                        let onkeypress = ctx.link().batch_callback(move |e: KeyboardEvent| {
                            if e.key() == "Enter" {
                                let input: HtmlInputElement = e.target_unchecked_into();
                                let val = input.value().parse::<i8>().unwrap();
                                Some(AppMsg::ScoreEnter(round_idx, player_idx, val))
                            } else {
                                None
                            }
                        });

                        let onclick = ctx.link().callback(move |_| {
                            AppMsg::ScoreEdit(round_idx, player_idx)
                        });

                        html! {
                            <td {onclick}>
                            {if score.is_editing {
                                html! {
                                    <input ref={node_ref} {onkeypress} value={if let Some(s) = score.val { s.to_string() } else { String::new() }} type="number"/>
                                }
                            } else {
                                html! {
                                    {if let Some(score) = score.val {
                                        let class = Classes::from("score");

                                        let percent: f64 = (score as f64 + self.state.negative_size as f64) / self.state.deck_size as f64;
                                        let color = self.gradient.at(percent).to_hex_string();
                                        let style = "color: ".to_owned() + &color;

                                        html! {
                                            <span {style} {class}>{score.to_string()}</span>
                                        }
                                    } else {
                                        html! {
                                            {"--"}
                                        }
                                    }}
                                }
                            }}
                            </td>
                        }
                    })}
                    </tr>
                }) }

                </table>
                <div class="button">
                    <button onclick={ctx.link().callback(move |_| AppMsg::GameNew)}>{"NEW GAME"}</button>
                </div>
                </div>


                    }

                } else {
                    let disabled = self.state.players.len() < 2;
                    html! {
                        <div>

                            <ul>
                                { for self.state.players.iter().enumerate().map(|(idx, player)| self.view_player(idx, player, ctx.link()))}
                            </ul>
                            {self.view_input(ctx.link())}

                            <div class="button">
                                <button {disabled} onclick={ctx.link().callback(move |_| AppMsg::GameStart)}>{"START GAME"}</button>
                            </div>
                        </div>
                    }

                }}
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
