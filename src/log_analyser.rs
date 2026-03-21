use winnow::{Parser, combinator::repeat, error::ContextError, token::take_until};

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
        "[main] ".void(),
        take_until(0.., " : "),
        " : ".void(),
        take_until(0.., "[main]"),
    )
        .map(|(_, name, _, content): ((), &str, (), &str)| (name, content.trim()))
        .parse_next(input)
}

pub fn analyse_log<'s>(log: &'s str) -> Result<Vec<(&'s str, &'s str)>, ContextError> {
    let mut trimmed = log.trim();
    let log: Vec<(&str, &str)> = repeat(0.., parse_a_html).parse_next(&mut trimmed)?;
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
            acc.push((cs.query_charactor(chara), content.to_string()));
        }
    }
}
