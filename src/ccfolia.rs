use std::time::Duration;
use thirtyfour::prelude::*;

pub async fn send_to_ccfolia(
    driver: &WebDriver,
    chara: String,
    content: String,
) -> WebDriverResult<()> {
    // 1. aria-label="キャラクター選択"を持つbuttonを押下
    println!("キャラクター選択を開始...");
    let char_select_btn = driver
        .find(By::Css("button[aria-label='キャラクター選択']"))
        .await?;
    char_select_btn.click().await?;

    // DOMの更新待ち
    println!("アニメーション待機中...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2〜4. class="MuiList-root css-1uzmcsd"を持つdivの子要素からキャラクターを探し、二つ上の親要素を押下
    // XPathではテキスト検索「text()='...'」と親要素の取得「/..」が一度に記述できます
    println!("切り替えボタン探索中...");
    let target_xpath = format!(
        "//div[contains(@role, 'button')]//*[text()='{}']/../..",
        chara
    );
    let grandparent_elem = driver.find(By::XPath(target_xpath)).await?;
    grandparent_elem.click().await?;

    println!("アニメーション待機中...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5〜6. id="downshift-:rm:-input"を持つtextareaに文字列を入力
    // CSSセレクタでは「:」のエスケープが必要で複雑になるため、ここでもXPathを使用するのが安全です
    println!("入力中...");
    let textarea = driver
        .find(By::XPath("//textarea[@id='downshift-:rm:-input']"))
        .await?;
    textarea.send_keys(&content).await?;

    // 7. type="submit"を持つボタンを押下
    println!("送信中...");
    let submit_btn = driver.find(By::Css("button[type='submit']")).await?;
    submit_btn.click().await?;

    println!("完了！");
    Ok(())
}
