mod log_analyser;

use gpui::{
    App, Application, Bounds, Context, Entity, Point, ScrollHandle, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_component::button;
use rfd::FileDialog;

use crate::log_analyser::{LogAnalyser, analyse_log};

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
                    .w(px(200.0))
                    .flex_shrink_0()
                    .h_full()
                    .bg(rgb(0x282c34))
                    .p_4()
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
                                    if let Ok(read) = std::fs::read_to_string(path) {
                                        if let Ok(analysed_data) = analyse_log(&read) {
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
                                    }
                                }
                            })),
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
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(800.), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
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
