extern crate html5ever;
extern crate tendril;
extern crate memmap;
#[macro_use]
extern crate string_cache;
#[macro_use]
extern crate lazy_static;

use html5ever::tokenizer::*;
use string_cache::{QualName, Namespace};
use tendril::StrTendril;

enum Sink {
    Ready,
    NextTextIsUser,
    GotUser(StrTendril),
    NextTextIsDate(StrTendril),
    GotUserAndDate(StrTendril, StrTendril),
    NextTextIsComment(StrTendril, StrTendril),
}

lazy_static! {
    static ref CLASS: QualName = QualName { ns: Namespace(atom!("")), local: atom!("class") };
}

impl TokenSink for Sink {
    fn process_token(&mut self, token: Token) {
        use std::mem::{swap, replace};
        let mut state = Sink::Ready;
        swap(self, &mut state);

        let new = match (state, token) {
            (Sink::Ready, Token::TagToken(Tag { kind: StartTag, attrs, .. })) => {
                if attrs.iter().any(|a| a.name == *CLASS && a.value == "user".into()) {
                    Sink::NextTextIsUser
                } else {
                    Sink::Ready
                }
            }
            (Sink::NextTextIsUser, Token::CharacterTokens(user)) => {
                Sink::GotUser(user)
            }
            (Sink::GotUser(user), Token::TagToken(Tag {kind: StartTag, attrs, .. })) => {
                if attrs.iter().any(|a| a.name == *CLASS && a.value == "meta".into()) {
                    Sink::NextTextIsDate(user)
                } else {
                    Sink::GotUser(user)
                }
            }
            (Sink::NextTextIsDate(user), Token::CharacterTokens(date)) => {
                Sink::GotUserAndDate(user, date)
            }
            (Sink::GotUserAndDate(user, date), Token::TagToken(Tag { kind: StartTag, name, .. })) => {
                if name == "p".into() {
                    Sink::NextTextIsComment(user, date)
                } else {
                    Sink::GotUserAndDate(user, date)
                }
            }
            (Sink::NextTextIsComment(user, date), Token::CharacterTokens(comment)) => {
                println!("{} at {}: {}", user, date, comment);
                Sink::Ready
            }
            (state, o) => {
                println!("{:?}", o);
                state
            }
        };

        replace(self, new);
    }
}

fn main() {
    let mut memmap = memmap::Mmap::open_path("./messages.htm", memmap::Protection::Read).unwrap();
    let string = unsafe { StrTendril::from_byte_slice_without_validating(memmap.as_slice()) };
    let mut tokenizer = Tokenizer::new(Sink::Ready, Default::default());
    tokenizer.feed(From::from(r#"
        <span class="user"> Kate </span>
        <span class="meta"> Today </span>
        <p>I forgot that I don&#039;t know how to flirt like at all</p>
    "#));
    //tokenizer.feed(string);
    tokenizer.run();
}
