//! Just enough of the MSVC name decoration to read FMOD's exports:
//! `?method@Class@FMOD@@QAG?AW4FMOD_RESULT@@<parameters>@Z`.

/// A decoded member function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub class: String,
    pub method: String,
    /// Parameter types after `this`, as C++ spells them.
    pub params: Vec<String>,
}

impl Member {
    /// `Class::method`.
    #[must_use]
    pub fn qualified(&self) -> String {
        format!("{}::{}", self.class, self.method)
    }
}

/// `Class::method(params)` for a decorated name, the name itself for a C
/// export (`FMOD_EventSystem_Create`).
#[must_use]
pub fn pretty(name: &str) -> String {
    match member(name) {
        Some(m) => format!("{}({})", m.qualified(), m.params.join(", ")),
        None => name.to_owned(),
    }
}

/// `Class::method` for a decorated name, the name itself for a C export.
#[must_use]
pub fn short(name: &str) -> String {
    member(name).map_or_else(|| name.to_owned(), |m| m.qualified())
}

/// Decodes a decorated member function name.
#[must_use]
pub fn member(name: &str) -> Option<Member> {
    let rest = name.strip_prefix('?')?;
    let mut p = Parser {
        s: rest.as_bytes(),
        at: 0,
        names: Vec::new(),
        args: Vec::new(),
    };
    let method = p.fragment()?;
    let class = p.fragment()?;
    // the namespace(s), then the terminating '@'
    while p.peek()? != b'@' {
        p.fragment()?;
    }
    p.at += 1;
    // access and storage (Q public, A no cv-qualifier on this), convention
    p.at += 2;
    if !matches!(p.next()?, b'G' | b'E' | b'A') {
        return None;
    }
    p.ty()?; // return type
    let mut params = Vec::new();
    if p.peek()? == b'X' {
        p.at += 1;
    } else {
        while p.peek()? != b'@' {
            params.push(p.param()?);
        }
    }
    Some(Member {
        class,
        method,
        params,
    })
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
    /// Name fragments for back-references `0`-`9`.
    names: Vec<String>,
    /// Parameter types for back-references `0`-`9`.
    args: Vec<String>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.at += 1;
        Some(c)
    }

    /// One name fragment: a back-reference digit or `text@`.
    fn fragment(&mut self) -> Option<String> {
        let c = self.peek()?;
        if c.is_ascii_digit() {
            self.at += 1;
            return self.names.get(usize::from(c - b'0')).cloned();
        }
        let end = self.s[self.at..].iter().position(|&b| b == b'@')?;
        let text = String::from_utf8_lossy(&self.s[self.at..self.at + end]).into_owned();
        self.at += end + 1;
        self.names.push(text.clone());
        Some(text)
    }

    /// A qualified name up to its closing `@`; the innermost name first.
    fn qualified(&mut self) -> Option<String> {
        let mut parts = Vec::new();
        while self.peek()? != b'@' {
            parts.push(self.fragment()?);
        }
        self.at += 1;
        parts.reverse();
        Some(parts.join("::").trim_start_matches("FMOD::").to_owned())
    }

    /// A parameter: a type, or a back-reference to an earlier one.
    fn param(&mut self) -> Option<String> {
        let c = self.peek()?;
        if c.is_ascii_digit() {
            self.at += 1;
            return self.args.get(usize::from(c - b'0')).cloned();
        }
        let start = self.at;
        let t = self.ty()?;
        if self.at - start > 1 {
            self.args.push(t.clone());
        }
        Some(t)
    }

    fn ty(&mut self) -> Option<String> {
        let c = self.next()?;
        Some(match c {
            b'X' => "void".into(),
            b'D' => "char".into(),
            b'E' => "unsigned char".into(),
            b'F' => "short".into(),
            b'G' => "unsigned short".into(),
            b'H' => "int".into(),
            b'I' => "unsigned int".into(),
            b'J' => "long".into(),
            b'K' => "unsigned long".into(),
            b'M' => "float".into(),
            b'N' => "double".into(),
            b'_' => match self.next()? {
                b'N' => "bool".into(),
                b'J' => "__int64".into(),
                b'K' => "unsigned __int64".into(),
                _ => return None,
            },
            b'?' => {
                // a class returned by value: ?A<type>
                self.next()?;
                self.ty()?
            }
            b'P' | b'Q' => {
                let cv = self.next()?;
                if cv == b'6' {
                    return self.function_pointer();
                }
                let inner = self.ty()?;
                if matches!(cv, b'B' | b'D') {
                    format!("const {inner}*")
                } else {
                    format!("{inner}*")
                }
            }
            b'V' | b'U' => self.qualified()?,
            b'W' => {
                self.next()?; // the enum's underlying size, 4
                self.qualified()?
            }
            _ => return None,
        })
    }

    /// `P6G<return><params>@Z`: kept as `callback(params)`. Its parameters
    /// share the back-reference table with the enclosing function's.
    fn function_pointer(&mut self) -> Option<String> {
        self.next()?; // convention
        self.ty()?;
        let mut params = Vec::new();
        if self.peek()? == b'X' {
            self.at += 1;
        } else {
            while self.peek()? != b'@' {
                params.push(self.param()?);
            }
            self.at += 1;
        }
        if self.next()? != b'Z' {
            return None;
        }
        Some(format!("callback({})", params.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_fmod_members() {
        assert_eq!(
            pretty("?setVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@M@Z"),
            "Event::setVolume(float)"
        );
        assert_eq!(
            pretty("?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ"),
            "Event::start()"
        );
        assert_eq!(
            pretty(
                "?load@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PBDPAUFMOD_EVENT_LOADINFO@@PAPAVEventProject@2@@Z"
            ),
            "EventSystem::load(const char*, FMOD_EVENT_LOADINFO*, EventProject**)"
        );
        assert_eq!(
            pretty("?getSubSound@Sound@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAV12@@Z"),
            "Sound::getSubSound(int, Sound**)"
        );
        assert_eq!(
            pretty("?getOpenState@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OPENSTATE@@PAIPA_N2@Z"),
            "Sound::getOpenState(FMOD_OPENSTATE*, unsigned int*, bool*, bool*)"
        );
        assert_eq!(
            pretty(
                "?playSound@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_CHANNELINDEX@@PAVSound@2@_NPAPAVChannel@2@@Z"
            ),
            "System::playSound(FMOD_CHANNELINDEX, Sound*, bool, Channel**)"
        );
        assert_eq!(
            pretty("?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z"),
            "EventSystem::init(int, unsigned int, void*, unsigned int)"
        );
        assert_eq!(pretty("FMOD_EventSystem_Create"), "FMOD_EventSystem_Create");
    }

    #[test]
    fn decodes_callbacks() {
        let m = member(
            "?setFileSystem@System@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PBDHPAIPAPAX2@ZP6G?AW43@PAX4@ZP6G?AW43@44I14@ZP6G?AW43@4I4@ZP6G?AW43@PAUFMOD_ASYNCREADINFO@@4@Z5H@Z",
        )
        .unwrap();
        assert_eq!(m.qualified(), "System::setFileSystem");
        assert_eq!(m.params.len(), 7);
        assert_eq!(
            m.params[0],
            "callback(const char*, int, unsigned int*, void**, void**)"
        );
        assert_eq!(m.params[5], "callback(void*, void*)");
        assert_eq!(m.params[6], "int");
    }
}
