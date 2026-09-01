//! Touch-oriented command bar for narrow terminals and software keyboards.

use crate::language::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TouchPage {
    #[default]
    Primary,
    Display,
    Tools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchContext {
    Editor,
    Help,
    Sound,
    Language,
    OpenPrompt,
    SavePrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchCommand {
    ShowHelp,
    CycleInput,
    Open,
    Save,
    NextPage,
    ToggleBigFont,
    TogglePageWidth,
    ToggleTheme,
    ShowLanguages,
    ShowSound,
    ToggleFocus,
    FontDec,
    FontInc,
    CycleLineSpacing,
    SaveAs,
    ToggleTouchMode,
    PrimaryPage,
    Previous,
    Next,
    Activate,
    Remove,
    Complete,
    Confirm,
    Cancel,
    ToggleHelpPreference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchButton {
    pub label: &'static str,
    pub command: TouchCommand,
}

const fn button(label: &'static str, command: TouchCommand) -> TouchButton {
    TouchButton { label, command }
}

pub fn buttons(context: TouchContext, page: TouchPage, language: Language) -> Vec<TouchButton> {
    use TouchCommand as Command;

    match context {
        TouchContext::Editor => editor_buttons(page, language),
        TouchContext::Help => match language {
            Language::Korean => vec![
                button("시작안내", Command::ToggleHelpPreference),
                button("닫기", Command::Cancel),
            ],
            Language::Japanese => vec![
                button("起動案内", Command::ToggleHelpPreference),
                button("閉じる", Command::Cancel),
            ],
            Language::English => vec![
                button("Startup", Command::ToggleHelpPreference),
                button("Close", Command::Cancel),
            ],
        },
        TouchContext::Sound => match language {
            Language::Korean => vec![
                button("이전", Command::Previous),
                button("전환", Command::Activate),
                button("다음", Command::Next),
                button("닫기", Command::Cancel),
            ],
            Language::Japanese => vec![
                button("前", Command::Previous),
                button("切替", Command::Activate),
                button("次", Command::Next),
                button("閉じる", Command::Cancel),
            ],
            Language::English => vec![
                button("Prev", Command::Previous),
                button("Toggle", Command::Activate),
                button("Next", Command::Next),
                button("Close", Command::Cancel),
            ],
        },
        TouchContext::Language => match language {
            Language::Korean => vec![
                button("이전", Command::Previous),
                button("사용", Command::Activate),
                button("다음", Command::Next),
                button("제거", Command::Remove),
                button("닫기", Command::Cancel),
            ],
            Language::Japanese => vec![
                button("前", Command::Previous),
                button("使用", Command::Activate),
                button("次", Command::Next),
                button("削除", Command::Remove),
                button("閉じる", Command::Cancel),
            ],
            Language::English => vec![
                button("Prev", Command::Previous),
                button("Use", Command::Activate),
                button("Next", Command::Next),
                button("Remove", Command::Remove),
                button("Close", Command::Cancel),
            ],
        },
        TouchContext::OpenPrompt => match language {
            Language::Korean => vec![
                button("이전", Command::Previous),
                button("완성", Command::Complete),
                button("다음", Command::Next),
                button("열기", Command::Confirm),
                button("취소", Command::Cancel),
            ],
            Language::Japanese => vec![
                button("前", Command::Previous),
                button("補完", Command::Complete),
                button("次", Command::Next),
                button("開く", Command::Confirm),
                button("取消", Command::Cancel),
            ],
            Language::English => vec![
                button("Prev", Command::Previous),
                button("Complete", Command::Complete),
                button("Next", Command::Next),
                button("Open", Command::Confirm),
                button("Cancel", Command::Cancel),
            ],
        },
        TouchContext::SavePrompt => match language {
            Language::Korean => vec![
                button("저장", Command::Confirm),
                button("취소", Command::Cancel),
            ],
            Language::Japanese => vec![
                button("保存", Command::Confirm),
                button("取消", Command::Cancel),
            ],
            Language::English => vec![
                button("Save", Command::Confirm),
                button("Cancel", Command::Cancel),
            ],
        },
    }
}

fn editor_buttons(page: TouchPage, language: Language) -> Vec<TouchButton> {
    use TouchCommand as Command;

    match (page, language) {
        (TouchPage::Primary, Language::Korean) => vec![
            button("도움", Command::ShowHelp),
            button("입력", Command::CycleInput),
            button("열기", Command::Open),
            button("저장", Command::Save),
            button("더보기", Command::NextPage),
        ],
        (TouchPage::Primary, Language::Japanese) => vec![
            button("ヘルプ", Command::ShowHelp),
            button("入力", Command::CycleInput),
            button("開く", Command::Open),
            button("保存", Command::Save),
            button("次", Command::NextPage),
        ],
        (TouchPage::Primary, Language::English) => vec![
            button("Help", Command::ShowHelp),
            button("Input", Command::CycleInput),
            button("Open", Command::Open),
            button("Save", Command::Save),
            button("More", Command::NextPage),
        ],
        (TouchPage::Display, Language::Korean) => vec![
            button("큰글", Command::ToggleBigFont),
            button("폭", Command::TogglePageWidth),
            button("테마", Command::ToggleTheme),
            button("언어", Command::ShowLanguages),
            button("소리", Command::ShowSound),
            button("다음", Command::NextPage),
        ],
        (TouchPage::Display, Language::Japanese) => vec![
            button("拡大", Command::ToggleBigFont),
            button("幅", Command::TogglePageWidth),
            button("テーマ", Command::ToggleTheme),
            button("言語", Command::ShowLanguages),
            button("音", Command::ShowSound),
            button("次", Command::NextPage),
        ],
        (TouchPage::Display, Language::English) => vec![
            button("Big", Command::ToggleBigFont),
            button("Page", Command::TogglePageWidth),
            button("Theme", Command::ToggleTheme),
            button("Lang", Command::ShowLanguages),
            button("Sound", Command::ShowSound),
            button("More", Command::NextPage),
        ],
        (TouchPage::Tools, Language::Korean) => vec![
            button("집중", Command::ToggleFocus),
            button("작게", Command::FontDec),
            button("크게", Command::FontInc),
            button("간격", Command::CycleLineSpacing),
            button("별도", Command::SaveAs),
            button("터치끔", Command::ToggleTouchMode),
            button("처음", Command::PrimaryPage),
        ],
        (TouchPage::Tools, Language::Japanese) => vec![
            button("集中", Command::ToggleFocus),
            button("縮小", Command::FontDec),
            button("拡大", Command::FontInc),
            button("行間", Command::CycleLineSpacing),
            button("別名", Command::SaveAs),
            button("タッチ切", Command::ToggleTouchMode),
            button("戻る", Command::PrimaryPage),
        ],
        (TouchPage::Tools, Language::English) => vec![
            button("Focus", Command::ToggleFocus),
            button("Size-", Command::FontDec),
            button("Size+", Command::FontInc),
            button("Line", Command::CycleLineSpacing),
            button("As", Command::SaveAs),
            button("Touch", Command::ToggleTouchMode),
            button("Back", Command::PrimaryPage),
        ],
    }
}

pub fn command_at(
    column: u16,
    width: u16,
    context: TouchContext,
    page: TouchPage,
    language: Language,
) -> Option<TouchCommand> {
    if width == 0 || column >= width {
        return None;
    }
    let buttons = buttons(context, page, language);
    let index = usize::from(column) * buttons.len() / usize::from(width);
    buttons.get(index).map(|button| button.command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_pages_expose_every_function_key_feature() {
        let primary = buttons(TouchContext::Editor, TouchPage::Primary, Language::English);
        let display = buttons(TouchContext::Editor, TouchPage::Display, Language::English);
        let tools = buttons(TouchContext::Editor, TouchPage::Tools, Language::English);
        let commands = primary
            .into_iter()
            .chain(display)
            .chain(tools)
            .map(|button| button.command)
            .collect::<Vec<_>>();

        for expected in [
            TouchCommand::ShowHelp,
            TouchCommand::CycleInput,
            TouchCommand::ToggleFocus,
            TouchCommand::ToggleBigFont,
            TouchCommand::TogglePageWidth,
            TouchCommand::ToggleTheme,
            TouchCommand::FontDec,
            TouchCommand::FontInc,
            TouchCommand::ShowLanguages,
            TouchCommand::ShowSound,
            TouchCommand::CycleLineSpacing,
            TouchCommand::SaveAs,
            TouchCommand::ToggleTouchMode,
        ] {
            assert!(commands.contains(&expected));
        }
    }

    #[test]
    fn hit_testing_divides_the_entire_bar_into_equal_buttons() {
        assert_eq!(
            command_at(
                0,
                40,
                TouchContext::Editor,
                TouchPage::Primary,
                Language::English
            ),
            Some(TouchCommand::ShowHelp)
        );
        assert_eq!(
            command_at(
                39,
                40,
                TouchContext::Editor,
                TouchPage::Primary,
                Language::English
            ),
            Some(TouchCommand::NextPage)
        );
        assert_eq!(
            command_at(
                40,
                40,
                TouchContext::Editor,
                TouchPage::Primary,
                Language::English
            ),
            None
        );
    }
}
