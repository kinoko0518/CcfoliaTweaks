mod ccfolia;

use gpui::{
    App, Application, Bounds, Entity, Point, ScrollHandle, Window, WindowOptions, div, prelude::*,
    px, rgb, size,
};
use gpui_component::scroll::ScrollableElement;
use thirtyfour::{DesiredCapabilities, WebDriver};

use crate::{ccfolia::send_to_ccfolia, log_analyser::LogAnalyser};

mod log_analyser;
use gpui_component::button;
use rfd::FileDialog;
use std::sync::OnceLock;
use std::{io::Write, process::Command};
use tokio::runtime::{Builder, Runtime};

// アプリケーション全体で使い回すTokioランタイム
fn tokio_rt() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    // new_current_thread() を new_multi_thread() に変更し、enable_all() を追加
    RT.get_or_init(|| Builder::new_multi_thread().enable_all().build().unwrap())
}

use crate::log_analyser::{analyse_copied_text_log, analyse_html_log};

struct CcfoliaTweaks {
    pub analyser: Entity<LogAnalyser>,
    pub current_page: usize,
    pub items_per_page: usize,
    pub scroll_handle: ScrollHandle,
}

impl Render for CcfoliaTweaks {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let analyser = self.analyser.read(cx);
        let total_items = analyser.log.len();

        // 全ページ数を計算（ログが0件の場合は1とする）
        let total_pages = if total_items == 0 {
            1
        } else {
            (total_items + self.items_per_page - 1) / self.items_per_page
        };

        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);

        div()
            .flex()
            .flex_row()
            .gap_3()
            .size_full()
            .bg(rgb(0x21252b))
            .child(
                // ========== 左パネル ==========
                div()
                    .w(px(300.0))
                    .flex_shrink_0()
                    .h_full()
                    .bg(rgb(0x282c34))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_color(rgb(0xffffff)).child("操作パネル"))
                    .child(
                        button::Button::new("load_html")
                            .label("HTML出力を読み込み")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let file_path = FileDialog::new()
                                    .add_filter("HTMLファイル", &["txt", "html"])
                                    .set_title("ファイルを選択してください")
                                    .set_directory("/")
                                    .pick_file();

                                if let Some(path) = file_path {
                                    match std::fs::read_to_string(&path) {
                                        Ok(read) => {
                                            match analyse_html_log(&read) {
                                                Ok(analysed_data) => {
                                                    this.analyser.update(cx, |analyser, cx| {
                                                        analyser.analyse(analysed_data);
                                                        cx.notify();
                                                    });
                                                    // 読み込み時はページとスクロールを一番上へリセット
                                                    this.current_page = 0;
                                                    this.scroll_handle.set_offset(Point {
                                                        x: px(0.),
                                                        y: px(0.),
                                                    });
                                                    cx.notify();
                                                }
                                                Err(e) => eprintln!("HTML解析エラー: {:?}", e),
                                            }
                                        }
                                        Err(e) => eprintln!("ファイル読み込みエラー: {:?}", e),
                                    }
                                } else {
                                    eprintln!("ファイルが選択されませんでした");
                                }
                            })),
                    )
                    .child(
                        button::Button::new("load_copied_text")
                            .label("コピーしたテキストを読み込み")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let file_path = FileDialog::new()
                                    .add_filter("テキストファイル", &["txt"])
                                    .set_title("ファイルを選択してください")
                                    .set_directory("/")
                                    .pick_file();

                                if let Some(path) = file_path {
                                    match std::fs::read_to_string(&path) {
                                        Ok(read) => {
                                            match analyse_copied_text_log(&read) {
                                                Ok(analysed_data) => {
                                                    this.analyser.update(cx, |analyser, cx| {
                                                        analyser.analyse(analysed_data);
                                                        cx.notify();
                                                    });
                                                    // 読み込み時はページとスクロールを一番上へリセット
                                                    this.current_page = 0;
                                                    this.scroll_handle.set_offset(Point {
                                                        x: px(0.),
                                                        y: px(0.),
                                                    });
                                                    cx.notify();
                                                }
                                                Err(e) => eprintln!("テキスト解析エラー: {:?}", e),
                                            }
                                        }
                                        Err(e) => eprintln!("ファイル読み込みエラー: {:?}", e),
                                    }
                                } else {
                                    eprintln!("ファイルが選択されませんでした");
                                }
                            })),
                    )
                    .child(
                        button::Button::new("dump_all")
                            .label("まとめて書き出し")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let chara = &this.analyser.read(cx).charactors.charactors;
                                match FileDialog::new().save_file() {
                                    Some(path) => {
                                        let mut file = std::fs::File::create(path).unwrap();
                                        for (i, c) in &this.analyser.read(cx).log {
                                            writeln!(file, "[main] {} : {}\n", chara[*i], c)
                                                .unwrap();
                                        }
                                    }
                                    None => {
                                        eprintln!("書き込み先が指定されませんでした");
                                    }
                                }
                            })),
                    )
                    .child(
                        button::Button::new("ccfolia_out")
                            .label("ccfolia出力開始")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let analyser = this.analyser.read(cx);
                                let charactors_list = analyser.charactors.charactors.clone();
                                let log_data = analyser.log.clone();

                                cx.background_executor()
                                    .spawn(async move {
                                        // Tokioのランタイムにタスクを投げる
                                        let result = tokio_rt()
                                            .spawn(async move {
                                                // Geckodriverへの接続
                                                println!("接続中...");
                                                let caps = DesiredCapabilities::firefox();
                                                let driver =
                                                    WebDriver::new("http://localhost:4444", caps)
                                                        .await
                                                        .unwrap();

                                                // 抽出しておいた（Send可能な）データを使用する
                                                for (i, c) in &log_data {
                                                    let _ = send_to_ccfolia(
                                                        &driver,
                                                        charactors_list[*i].clone(),
                                                        c.clone(),
                                                    )
                                                    .await;
                                                }

                                                let _ = driver.quit().await;
                                            })
                                            .await;

                                        if let Err(e) = result {
                                            eprintln!("Tokio task failed: {:?}", e);
                                        }
                                    })
                                    .detach();
                            })),
                    )
                    .child(div().flex_grow())
                    .child(
                        div()
                            .overflow_y_scrollbar()
                            .text_color(rgb(0xffffff))
                            .child("要求キャラクター")
                            .children(
                                self.analyser
                                    .read(cx)
                                    .charactors
                                    .charactors
                                    .iter()
                                    .map(|chara| format!("＊{}", chara)),
                            ),
                    ),
            )
            .child(
                // ========== 右パネル ==========
                div().flex_grow().overflow_hidden().h_full().p_4().child(
                    // ScrollHandleを紐付け、静的なIDで状態を管理する
                    div()
                        .id("log-scroll-area")
                        .track_scroll(&self.scroll_handle.clone())
                        .size_full()
                        .overflow_y_scroll()
                        .children({
                            let mut items = Vec::new();

                            // ========== 上端の操作（前のページへ） ==========
                            if self.current_page > 0 {
                                items.push(
                                    div().w_full().flex().justify_center().py_4().child(
                                        button::Button::new("prev_page")
                                            .label(format!(
                                                "前の{}件を読み込む (現在: {}/{})",
                                                self.items_per_page,
                                                self.current_page + 1,
                                                total_pages
                                            ))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.current_page -= 1;
                                                // 戻った時はスクロールを一番下にする（GPUIのレイアウトエンジンが次フレームで新しいMax値にクランプする）
                                                this.scroll_handle.set_offset(Point {
                                                    x: px(0.),
                                                    y: px(999999.),
                                                });
                                                cx.notify();
                                            })),
                                    ),
                                );
                            } else if total_items > 0 {
                                items.push(
                                    div()
                                        .w_full()
                                        .flex()
                                        .justify_center()
                                        .py_4()
                                        .text_color(rgb(0x888888))
                                        .child(format!(
                                            "ページ {}/{}",
                                            self.current_page + 1,
                                            total_pages
                                        )),
                                );
                            }

                            // ========== ログの描画 ==========
                            if total_items > 0 {
                                for idx in start_idx..end_idx {
                                    let (char_idx, text) = &analyser.log[idx];
                                    let char_name = analyser
                                        .charactors
                                        .charactors
                                        .get(*char_idx)
                                        .cloned()
                                        .unwrap_or_else(|| "Unknown".to_string());

                                    items.push(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .mb_4()
                                            .child(div().text_color(rgb(0x61afef)).child(char_name))
                                            .child(
                                                div()
                                                    .text_color(rgb(0xffffff))
                                                    .w_full()
                                                    .child(text.clone()),
                                            ),
                                    );
                                }
                            } else {
                                items.push(
                                    div().text_color(rgb(0x888888)).child("ログがありません"),
                                );
                            }

                            // ========== 下端の操作（次のページへ） ==========
                            if self.current_page + 1 < total_pages {
                                items.push(
                                    div().w_full().flex().justify_center().py_4().child(
                                        button::Button::new("next_page")
                                            .label(format!(
                                                "次の{}件を読み込む (現在: {}/{})",
                                                self.items_per_page,
                                                self.current_page + 1,
                                                total_pages
                                            ))
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.current_page += 1;
                                                // 進んだ時はスクロールを一番上にする
                                                this.scroll_handle.set_offset(Point {
                                                    x: px(0.),
                                                    y: px(0.),
                                                });
                                                cx.notify();
                                            })),
                                    ),
                                );
                            }

                            items
                        }),
                ),
            )
    }
}

fn main() {
    Command::new("firefox")
        .env("MOZ_MARIONETTE", "1")
        .arg("--marionette")
        .arg("--no-remote")
        .arg("--new-instance")
        .spawn().expect("Firefoxマリオネットの起動に失敗しました。firefoxのバイナリにパスが通っていることを確認してください");

    Command::new("geckodriver")
        .arg("--connect-existing")
        .arg("--marionette-port")
        .arg("2828")
        .arg("--port")
        .arg("4444")
        .spawn()
        .expect("geckdriverの起動に失敗しました。");

    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                let analyser = cx.new(|_| LogAnalyser::default());

                cx.new(|cx| {
                    cx.observe(&analyser, |this: &mut CcfoliaTweaks, _, cx| {
                        this.current_page = 0;
                        this.scroll_handle.set_offset(Point {
                            x: px(0.),
                            y: px(0.),
                        });
                        cx.notify()
                    })
                    .detach();

                    CcfoliaTweaks {
                        analyser,
                        current_page: 0,
                        items_per_page: 100,
                        scroll_handle: ScrollHandle::new(), // ScrollHandleの初期化
                    }
                })
            },
        )
        .unwrap();
    });
}
