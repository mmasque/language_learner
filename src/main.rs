const TARGET_LANGUAGE: &str = "Dutch";
const KNOWN_LANGUAGE: &str = "English";

enum Answer {
    Correct,
    Incorrect,
}

impl From<bool> for Answer {
    fn from(correct: bool) -> Self {
        match correct {
            true => Self::Correct,
            false => Self::Incorrect,
        }
    }
}

#[derive(Debug)]
struct Word {
    id: u64,
    target: String,
    known: String,
    rating: f64,
}

impl Word {
    fn new(id: u64, target: String, known: String) -> Word {
        Word {
            id,
            target,
            known,
            rating: 0.0,
        }
    }

    fn update_rating(&mut self, answer: Answer) {
        let rating_change = match answer {
            Answer::Correct => 10.0,
            Answer::Incorrect => -10.0,
        };
        self.rating = f64::max(0.0, f64::min(self.rating + rating_change, 100.0));
    }

    // For now always target -> known
    fn display(&self) {
        println!(
            "Translate the following word from {TARGET_LANGUAGE} into {KNOWN_LANGUAGE}: {}",
            self.target
        );
    }

    // Process response
    fn response_matches_known(&self, response: &str) -> Answer {
        (response == self.known).into()
    }
}

fn init_words() -> Vec<Word> {
    let hoi = Word::new(0, "hoi".to_string(), "hello".to_string());
    let hij = Word::new(1, "hij".to_string(), "he".to_string());
    let zij = Word::new(2, "zij".to_string(), "she".to_string());
    let inderdaad = Word::new(3, "inderdaad".to_string(), "indeed".to_string());
    vec![hoi, hij, zij, inderdaad]
}

fn read_string() -> String {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Cannot read user input");
    // Remove newline
    input.pop();
    input
}

fn main() {
    // Load the data
    let mut words = init_words();
    let mut practiced_words = vec![];
    println!("Welcome to language learning!");

    loop {
        // Pop, give, process response, add to list, sort by
        if let Some(mut word) = words.pop() {
            word.display();
            let response_outcome = word.response_matches_known(&read_string());
            word.update_rating(response_outcome);
            practiced_words.push(word);
        } else {
            // practiced all the words, remake the list
            words = practiced_words;
            words.reverse(); // popping and then pushing reverses the order we started withs
            words.sort_by(|v, w| w.rating.total_cmp(&v.rating));
            practiced_words = vec![];
        }
    }
}
