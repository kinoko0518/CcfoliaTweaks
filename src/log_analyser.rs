use winnow::{
    Parser,
    ascii::digit1,
    combinator::{alt, eof, peek, repeat, repeat_till},
    error::ContextError,
    token::{any, take_until},
};

#[derive(Default)]
pub struct LogAnalyser {
    pub charactors: CharactorSet,
    pub log: Vec<(usize, String)>,
}

#[derive(Default)]
pub struct CharactorSet {
    pub charactors: Vec<String>,
}

fn parse_a_html<'s>(input: &mut &'s str) -> Result<(&'s str, &'s str), ContextError> {
    (
        "[main]".void(),
        take_until(0.., ':'),
        ':'.void(),
        repeat_till(0.., any.void(), peek(alt(("[main]".void(), eof.void()))))
            .map(|(_, _): ((), _)| ())
            .take(),
    )
        .map(|(_, name, _, content): ((), &str, (), &str)| (name.trim(), content.trim()))
        .parse_next(input)
}

pub fn analyse_html_log<'s>(log: &'s str) -> Result<Vec<(&'s str, &'s str)>, ContextError> {
    let mut trimmed = log.trim();
    let log: Vec<(&str, &str)> = repeat(0.., parse_a_html).parse_next(&mut trimmed)?;
    Ok(log)
}

fn copied_text_nameplate<'s>(input: &mut &'s str) -> Result<&'s str, ContextError> {
    (
        take_until(0.., " - ").verify(|s: &str| !s.contains('\n') && !s.contains('\r')),
        " - ".void(),
        alt((
            (
                alt(("今日".void(), "昨日".void(), ("先週 ", any, "曜日").void())),
                ' ',
                (digit1, ':', digit1),
            )
                .void(),
            (digit1, '/', digit1, '/', digit1).void(),
        )),
    )
        .map(|(name, _, _): (&'s str, (), ())| name)
        .parse_next(input)
}

fn parse_a_copied_text<'s>(input: &mut &'s str) -> Result<(&'s str, &'s str), ContextError> {
    (
        copied_text_nameplate,
        repeat_till(
            0..,
            any.void(),
            peek(alt((copied_text_nameplate.void(), eof.void()))),
        )
        .map(|(_, _): ((), _)| ())
        .take(),
    )
        .map(|(nameplate, content): (&'s str, &'s str)| (nameplate, content.trim()))
        .parse_next(input)
}

pub fn analyse_copied_text_log<'s>(log: &'s str) -> Result<Vec<(&'s str, &'s str)>, ContextError> {
    let mut trimmed = log.trim();
    let log: Vec<(&'s str, &'s str)> = repeat(0.., parse_a_copied_text).parse_next(&mut trimmed)?;
    Ok(log)
}

impl<'s> CharactorSet {
    fn query_charactor(&mut self, name: &'s str) -> usize {
        for (i, c) in self.charactors.iter().enumerate() {
            if *c == name {
                return i;
            }
        }
        self.charactors.push(name.to_string());
        return self.charactors.len() - 1;
    }
}

impl LogAnalyser {
    pub fn analyse(&mut self, value: Vec<(&str, &str)>) {
        let cs = &mut self.charactors;
        let acc = &mut self.log;

        for (chara, content) in value {
            let mut content = content;
            const EDITED: &str = "[編集済]";
            if content.ends_with(EDITED) {
                content = &content[0..(content.len() - EDITED.len())];
            }
            if !content.is_empty() {
                acc.push((cs.query_charactor(chara), content.trim().to_string()));
            }
        }
    }
}
