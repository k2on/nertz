use gloo::storage::{LocalStorage, Storage};
use state::{Player, PlayerId, Round, Score, State};
use strum::IntoEnumIterator;
use web_sys::{HtmlInputElement, Node};
use yew::events::{FocusEvent, KeyboardEvent};
use yew::html::Scope;
use yew::{
    classes, html, use_state, Callback, Classes, Component, Context, Html, NodeRef, Properties,
    TargetCast,
};

mod state;

const KEY: &str = "yew.nertzpro.self";

pub enum Msg {
    PlayerAdd(String),
    PlayerRemove(usize),
    ScoreEnter(usize, usize, i8),
    GameStart,
    GameNew,
}

pub struct App {
    state: State,
    focus_ref: NodeRef,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let state = LocalStorage::get(KEY).unwrap_or_else(|_| State::new());
        let focus_ref = NodeRef::default();
        Self { state, focus_ref }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::PlayerAdd(name) => {
                if !name.is_empty() {
                    let player = Player {
                        id: self.state.players.len() as u32,
                        name,
                    };
                    self.state.players.push(player);
                }
            }
            Msg::PlayerRemove(idx) => {
                self.state.player_remove(idx);
            }
            Msg::GameStart => {
                self.state.is_game_started = true;
                self.state.next_round();
            }
            Msg::ScoreEnter(round_idx, player_idx, score) => {
                self.state.rounds[round_idx].scores[player_idx] = Some(score);
            }
            Msg::GameNew => {
                self.state = State::new();
            }
        }
        LocalStorage::set(KEY, &self.state).expect("failed to set");
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="nertzpro">
                <img id="logo" src="static/logo.png" alt="NERTS.PRO"/>
                { if self.state.is_game_started {
                    html! {
                        <>
                            <table class="scores">
                                <tr>
                                { for self.state.players.iter().map(|player| html! { <td>{player.name.clone().chars().nth(0).unwrap().to_uppercase()}</td> }) }
                                </tr>
                                { for self.state.rounds.iter().enumerate().map(|(i, round)| html! {
                                    <tr>
                                        { for round.scores.iter().enumerate().map(|(j, score)| html! { <ComponentScore update_score={ctx.link().callback(|(r, p, s)| Msg::ScoreEnter(r, p, s) )} round_idx={i} player_idx={j} score={score}/> }) }
                                        // { for round.scores.iter().enumerate().map(|(j, score)| html! { <ComponentScore update_score={Callback::from(|(round, player, score)| Msg::ScoreEnter(round, player, score))} round_idx={i} player_idx={j} score={score}/> }) }
                                    </tr>
                                }) }
                            </table>
                            { self.view_new_game_button(ctx.link()) }
                        </>
                    }
                } else {
                    html! {
                        <>
                            <ul class="todo-list">
                                { for self.state.players.iter().enumerate().map(|e| self.view_entry(e, ctx.link())) }
                            </ul>
                            { self.view_input(ctx.link()) }
                            <div>
                                { self.view_start_game_button(ctx.link()) }
                            </div>
                        </>
                    }
                } }
            </div>
        }
    }
}

impl App {
    fn view_input(&self, link: &Scope<Self>) -> Html {
        let onkeypress = link.batch_callback(|e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                let value = input.value();
                input.set_value("");
                Some(Msg::PlayerAdd(value))
            } else {
                None
            }
        });
        html! {
            <input
                class="new-player"
                placeholder="Player Name"
                {onkeypress}
            />
        }
    }

    fn view_entry(&self, (idx, player): (usize, &Player), link: &Scope<Self>) -> Html {
        let mut class = Classes::from("todo");
        html! {
            <li {class}>
                <div class="view">
                    <label>{ &player.name }</label>
                    <button class="destroy" onclick={link.callback(move |_| Msg::PlayerRemove(idx))}>{"x"}</button>
                </div>
            </li>
        }
    }

    fn view_start_game_button(&self, link: &Scope<Self>) -> Html {
        html! {
            <button class="start" onclick={link.callback(move |_| Msg::GameStart)}>{"START GAME"}</button>
        }
    }

    fn view_new_game_button(&self, link: &Scope<Self>) -> Html {
        html! {
            <button class="new" onclick={link.callback(move |_| Msg::GameNew)}>{"new game"}</button>
        }
    }
}

struct ComponentScore {
    props: PropsScore,
    editing: bool,
    input_ref: NodeRef,
}

#[derive(Properties, Clone, PartialEq)]
struct PropsScore {
    score: Score,
    round_idx: usize,
    player_idx: usize,
    update_score: Callback<(usize, usize, i8)>,
}

enum ScoreMsg {
    ToggleEditing,
    UpdateScore(i8),
}

impl Component for ComponentScore {
    type Message = ScoreMsg;
    type Properties = PropsScore;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            props: ctx.props().clone(),
            editing: false,
            input_ref: NodeRef::default(),
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if let Some(input) = self.input_ref.cast::<HtmlInputElement>() {
            if self.editing {
                input.focus();
            }
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ScoreMsg::ToggleEditing => {
                self.editing = true;
                true
            }
            ScoreMsg::UpdateScore(val) => {
                self.props.score = Some(val);
                self.editing = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onclick = ctx.link().callback(|_| ScoreMsg::ToggleEditing);

        let update_score = self.props.update_score.clone();
        let round_idx = self.props.round_idx;
        let player_idx = self.props.player_idx;

        let onkeypress = ctx.link().batch_callback(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let input: HtmlInputElement = e.target_unchecked_into();
                let value = input.value().parse::<i8>().unwrap();

                update_score.emit((round_idx, player_idx, value));

                Some(ScoreMsg::UpdateScore(value))
            } else {
                None
            }
        });

        html! {
            <td {onclick}>
                {
                    if self.editing {
                        html! { <input {onkeypress} ref={self.input_ref.clone()} type="number" /> }
                    } else {
                        if let Some(score) = self.props.score {
                            let mut class = Classes::from("score");
                            if score < 0 {
                                class.push("red");
                            }
                            html! {
                                <span {class}>{score.to_string()}</span>
                            }
                        } else {
                            html! {
                                <span>{"--"}</span>
                            }
                        }
                    }
                }
            </td>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
