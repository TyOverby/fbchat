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
use std::sync::mpsc::{sync_channel, SyncSender, IntoIter};
use std::iter::Peekable;


lazy_static! {
    static ref CLASS: QualName = QualName { ns: Namespace(atom!("")), local: atom!("class") };
}

struct Sender(SyncSender<Token>);
impl TokenSink for Sender {
    fn process_token(&mut self, token: Token) {
        self.0.send(token).unwrap();
    }
}

fn start_processing(input: &'static [u8]) -> IntoIter<Token> {
    use std::thread::spawn;
    let (sx, rx) = sync_channel(10);
    spawn(move || {
        let string = unsafe { StrTendril::from_byte_slice_without_validating(input) };
        let mut tokenizer = Tokenizer::new(Sender(sx), Default::default());
        tokenizer.feed(string);
    });

    rx.into_iter()
}


fn consume<I: Iterator<Item=Token>>(iter: I) {
    fn has_class(token: &Token, class: &StrTendril) -> bool {
        if let &Token::TagToken(Tag {kind: StartTag, ref attrs, ..}) = token {
            attrs.iter().any(|a| a.name == *CLASS && &a.value == class)
        } else {
            false
        }
    }
    fn extract_content<I: Iterator<Item=Token>>(mut iter: &mut Peekable<I>) -> Option<StrTendril> {
        fn is_character_tokens(t: Token) -> Option<StrTendril> {
            if let Token::CharacterTokens(s) = t {
                Some(s)
            } else {
                None
            }
        }

        let mut first = match iter.by_ref().filter_map(is_character_tokens).next() {
            Some(s) => s,
            None => return None,
        };

        while let Some(&Token::CharacterTokens(_)) = iter.peek() {
            if let Some(Token::CharacterTokens(next)) = iter.next() {
                first.push_tendril(&next);
            }
        }
        Some(first)
    }
    fn extract_user<I: Iterator<Item=Token>>(mut iter: &mut Peekable<I>) -> Option<StrTendril> {
        // Move the iterator until we are past a user
        let user_tendril = "user".into();
        if iter.by_ref().filter(|t| has_class(t, &user_tendril)).next().is_none() {
            return None;
        }
        extract_content(iter)
    }

    fn extract_date<I: Iterator<Item=Token>>(mut iter: &mut Peekable<I>) -> Option<StrTendril> {
        // Move the iterator until we are past a date tag
        let date_tendril = "meta".into();
        if iter.by_ref().filter(|t| has_class(t, &date_tendril)).next().is_none() {
            return None;
        }
        extract_content(iter)
    }

    fn extract_comment<I: Iterator<Item=Token>>(mut iter: &mut Peekable<I>) -> Option<StrTendril> {
        fn is_paragraph(t: &Token) -> bool {
            if let &Token::TagToken(Tag {kind: StartTag, name: atom!("p"), ..}) = t {
                true
            } else {
                false
            }
        }
        if iter.by_ref().filter(is_paragraph).next().is_none() {
            return None;
        }
        extract_content(iter)
    }

    let mut iter = iter.peekable();

    loop {
        let name = extract_user(&mut iter);
        let date = extract_date(&mut iter);
        let comment = extract_comment(&mut iter);
        match (name, date, comment) {
            (Some(name), Some(date), Some(comment)) => {
                println!("{} on {}: {}", name, date, comment);
            }
            (a, b, c) => {
                println!("{:?} {:?} {:?}", a, b, c);
                break;
            },
        }
    }
}

fn main() {
    let memmap = memmap::Mmap::open_path("./messages.htm", memmap::Protection::Read).unwrap();
    let token_iter = start_processing(
    br#"
        <span class="user"> Kelly </span>
        <span class="meta"> Today </span>
        <p>I forgot that I don&#039;t know how to flirt like at all</p>
    "#);
    let token_iter = start_processing(unsafe {::std::mem::transmute(memmap.as_slice())});
    consume(token_iter);
    ::std::mem::forget(memmap);
}
