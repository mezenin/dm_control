//! A closed command vocabulary, with Russian and English aliases.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Volume(i8),
    SetVolume(u8),
    Mute(bool),
    Brightness(i8),
    SetBrightness(u8),
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    LightToggle,
    LightPower(bool),
    LightMode,
    Sleep,
    Lock,
    Cancel,
}

fn normalized(text: &str) -> String {
    text.to_lowercase()
        .replace('ё', "е")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '%' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn parse(text: &str, wake_phrase: &str) -> Option<Action> {
    if text.contains('-') {
        return None;
    }
    let text = normalized(text);
    let wake = normalized(wake_phrase);
    let text = text.strip_prefix(&(wake + " ")).unwrap_or(&text);
    let text = ["хеллоу хекс ", "хелло хекс ", "хекс ", "hex "]
        .iter()
        .find_map(|p| text.strip_prefix(p))
        .unwrap_or(text);
    let text = text.strip_prefix("пожалуйста ").unwrap_or(text);
    let text = text.strip_suffix(" пожалуйста").unwrap_or(text);
    Some(match text {
        "громче"
        | "сделай громче"
        | "увеличь громкость"
        | "прибавь звук"
        | "прибавь громкость"
        | "volume up"
        | "louder" => Action::Volume(10),
        "тише"
        | "сделай тише"
        | "уменьши громкость"
        | "убавь звук"
        | "убавь громкость"
        | "volume down"
        | "quieter" => Action::Volume(-10),
        "выключи звук" | "отключи звук" | "без звука" | "mute" | "mute sound" => {
            Action::Mute(true)
        }
        "включи звук" | "верни звук" | "unmute" => Action::Mute(false),
        "ярче" | "сделай ярче" | "увеличь яркость" | "brightness up" | "brighter" => {
            Action::Brightness(10)
        }
        "темнее" | "сделай темнее" | "уменьши яркость" | "brightness down" | "dimmer" => {
            Action::Brightness(-10)
        }
        "пауза" | "поставь на паузу" | "останови музыку" | "pause" | "pause music" => {
            Action::Pause
        }
        "продолжи"
        | "продолжи воспроизведение"
        | "включи музыку"
        | "play"
        | "play music"
        | "resume" => Action::Play,
        "плей пауза" | "переключи воспроизведение" | "play pause" => {
            Action::PlayPause
        }
        "следующий трек" | "следующая песня" | "next track" | "next" => {
            Action::Next
        }
        "предыдущий трек" | "предыдущая песня" | "previous track" | "previous" => {
            Action::Previous
        }
        "переключи подсветку" | "переключи свет" | "toggle lights" => {
            Action::LightToggle
        }
        "включи подсветку" | "включи свет" | "lights on" => {
            Action::LightPower(true)
        }
        "выключи подсветку" | "выключи свет" | "lights off" => {
            Action::LightPower(false)
        }
        "смени режим подсветки" | "переключи режим подсветки" | "change light mode" => {
            Action::LightMode
        }
        "усыпи компьютер"
        | "усыпи мак"
        | "компьютер спать"
        | "переведи компьютер в сон"
        | "sleep computer"
        | "sleep mac" => Action::Sleep,
        "заблокируй компьютер" | "заблокируй мак" | "lock computer" | "lock mac" => {
            Action::Lock
        }
        "отмена" | "отмени" | "cancel" | "never mind" => Action::Cancel,
        _ => return parse_level(text),
    })
}

fn parse_level(text: &str) -> Option<Action> {
    for (prefix, volume) in [
        ("громкость ", true),
        ("установи громкость ", true),
        ("поставь громкость ", true),
        ("volume ", true),
        ("set volume ", true),
        ("яркость монитора ", false),
        ("яркость ", false),
        ("установи яркость ", false),
        ("brightness ", false),
        ("set brightness ", false),
    ] {
        if let Some(tail) = text.strip_prefix(prefix) {
            let tail = tail.trim_end_matches('%').trim();
            let tail = [" процентов", " процента", " процент", " percent"]
                .iter()
                .find_map(|s| tail.strip_suffix(s))
                .unwrap_or(tail);
            let n = number(tail)?;
            return Some(if volume {
                Action::SetVolume(n)
            } else {
                Action::SetBrightness(n)
            });
        }
    }
    None
}

fn number(text: &str) -> Option<u8> {
    if text.bytes().all(|c| c.is_ascii_digit()) {
        return text.parse::<u8>().ok().filter(|n| *n <= 100);
    }
    let mut total = 0_u16;
    let mut previous = 101;
    let mut count = 0;
    for word in text.split_whitespace() {
        let n = match word {
            "ноль" | "нуль" | "zero" => 0,
            "один" | "одна" | "one" => 1,
            "два" | "две" | "two" => 2,
            "три" | "three" => 3,
            "четыре" | "four" => 4,
            "пять" | "five" => 5,
            "шесть" | "six" => 6,
            "семь" | "seven" => 7,
            "восемь" | "eight" => 8,
            "девять" | "nine" => 9,
            "десять" | "ten" => 10,
            "одиннадцать" | "eleven" => 11,
            "двенадцать" | "twelve" => 12,
            "тринадцать" | "thirteen" => 13,
            "четырнадцать" | "fourteen" => 14,
            "пятнадцать" | "fifteen" => 15,
            "шестнадцать" | "sixteen" => 16,
            "семнадцать" | "seventeen" => 17,
            "восемнадцать" | "eighteen" => 18,
            "девятнадцать" | "nineteen" => 19,
            "двадцать" | "twenty" => 20,
            "тридцать" | "thirty" => 30,
            "сорок" | "forty" => 40,
            "пятьдесят" | "fifty" => 50,
            "шестьдесят" | "sixty" => 60,
            "семьдесят" | "seventy" => 70,
            "восемьдесят" | "eighty" => 80,
            "девяносто" | "ninety" => 90,
            "сто" | "hundred" => 100,
            _ => return None,
        };
        if count > 0
            && !(count == 1
                && (20..100).contains(&previous)
                && previous % 10 == 0
                && (1..10).contains(&n))
        {
            return None;
        }
        previous = n;
        total += n;
        count += 1;
    }
    (count > 0 && total <= 100).then_some(total as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_media_levels_and_configured_wake_phrase() {
        for (text, action) in [
            ("Hello Hex, сделай тише!", Action::Volume(-10)),
            ("Громкость пятьдесят пять процентов.", Action::SetVolume(55)),
            ("яркость 70%", Action::SetBrightness(70)),
            ("Включи подсветку", Action::LightPower(true)),
            ("выключи подсветку", Action::LightPower(false)),
            ("поставь на паузу пожалуйста", Action::Pause),
            ("усыпи компьютер", Action::Sleep),
        ] {
            assert_eq!(parse(text, "Hello Hex"), Some(action), "{text}");
        }
        assert_eq!(
            parse("Hey Computer, volume 40", "Hey Computer"),
            Some(Action::SetVolume(40))
        );
    }
    #[test]
    fn unknown_ambiguous_and_out_of_range_commands_do_nothing() {
        for text in [
            "",
            "почему компьютер спать",
            "не усыпи компьютер",
            "sleep",
            "volume 101",
            "volume -10",
            "volume one two",
            "громкость пятьдесят двадцать",
            "яркость минус десять",
            "открой терминал; rm rf",
        ] {
            assert_eq!(parse(text, "Hello Hex"), None, "{text}");
        }
    }
}
