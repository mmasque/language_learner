use std::error::Error;

use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantObject, CreateAssistantRequestArgs, CreateMessageRequestArgs,
        CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent, RunStatus, ThreadObject,
    },
    Client,
};

const TARGET_LANGUAGE: &str = "Dutch";
const KNOWN_LANGUAGE: &str = "English";

#[derive(PartialEq)]
enum Answer {
    Correct,
    Incorrect(String),
}

struct Game {
    client: Client<OpenAIConfig>,
    generating_assistant: AssistantObject,
    checking_assistant: AssistantObject,
    thread: ThreadObject,
}

impl Game {
    async fn new() -> Game {
        let client = Client::new();
        // Create an assistant that generates dutch words
        let assistant_request = CreateAssistantRequestArgs::default()
            .model("gpt-3.5-turbo")
            .instructions(
                "On input of a single english word,
                return an english sentence containing this word. It should be translateable
                into dutch by a B1 level student.",
            )
            .build()
            .unwrap();
        let generating_assistant = client.assistants().create(assistant_request).await.unwrap();

        // Create an assistant that checks translations
        let assistant_request = CreateAssistantRequestArgs::default()
        .model("gpt-3.5-turbo")
        .instructions(
            "You are a dutch teacher. The student will input an english sentence together with a dutch translation
            attempt of the sentence.
            Provide concise feedback on their translation attempt, address every mistake.",
        )
        .build()
        .unwrap();
        let checking_assistant = client.assistants().create(assistant_request).await.unwrap();

        // Create a thread (for now, we only ever have one user, so only one thread needed)
        let thread_request = CreateThreadRequestArgs::default().build().unwrap();
        let thread = client.threads().create(thread_request).await.unwrap();

        Game {
            client,
            generating_assistant,
            checking_assistant,
            thread,
        }
    }

    async fn get_text_from_assistant(&self, assistant_id: &str, input: &str) -> String {
        // A message
        let message = CreateMessageRequestArgs::default()
            .role("user")
            .content(input)
            .build()
            .unwrap();
        let _message_obj = self
            .client
            .threads()
            .messages(&self.thread.id)
            .create(message)
            .await
            .unwrap();

        // Make a run
        let run_request = CreateRunRequestArgs::default()
            .assistant_id(assistant_id)
            .build()
            .unwrap();
        let run = self
            .client
            .threads()
            .runs(&self.thread.id)
            .create(run_request)
            .await
            .unwrap();

        //wait for the run to complete
        loop {
            //retrieve the run
            let run = self
                .client
                .threads()
                .runs(&self.thread.id)
                .retrieve(&run.id)
                .await
                .unwrap();

            //check the status of the run
            match run.status {
                RunStatus::Completed => {
                    // once the run is completed we
                    // get the response from the run
                    // which will be the first message
                    // in the thread

                    //retrieve the response from the run
                    let response = self
                        .client
                        .threads()
                        .messages(&self.thread.id)
                        .list(&[("limit", "1")]) // ??
                        .await
                        .unwrap();
                    //get the message id from the response
                    let message_id = response.data.get(0).unwrap().id.clone();
                    //get the message from the response
                    let message = self
                        .client
                        .threads()
                        .messages(&self.thread.id)
                        .retrieve(&message_id)
                        .await
                        .unwrap();
                    //get the content from the message
                    let content = message.content.get(0).unwrap();
                    //return the text from the content
                    break match content {
                        MessageContent::Text(text) => text.text.value.clone(),
                        _ => {
                            panic!("not text aaa");
                        }
                    };
                }
                RunStatus::Queued | RunStatus::InProgress | RunStatus::Cancelling => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
                _ => panic!("Something bad happened with the OpenAI call"),
            }
        }
    }

    async fn resolve_answer(&mut self, word: &Word, answer: &Answer) {
        if answer == &Answer::Correct {
            println!("Well done!");
        } else {
            println!(
                "That isn't quite right, the answer is \"{}\". Let's try an exercise...",
                &word.known
            );
            let known_and_target = word.known.clone() + &word.target;
            let text_to_translate = self
                .get_text_from_assistant(&self.generating_assistant.id, &known_and_target)
                .await;
            println!("Here is an english sentence: {text_to_translate}");
            println!(
                "Provide a translation of the sentence in dutch using the word {}",
                word.target
            );
            // TODO check it actually has the word, make sure GPT doesnt complain about not using the word.

            let translation_attempt = read_string();
            println!("Checking your answer...");
            let translation_attempt_input = format!(
                "English sentence: {}\n Student's translation attempt: {}",
                text_to_translate, translation_attempt
            );
            let translation_feedback = self
                .get_text_from_assistant(&self.checking_assistant.id, &translation_attempt_input)
                .await;
            println!("{translation_feedback}");
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

    fn update_rating(&mut self, answer: &Answer) {
        let rating_change = match answer {
            Answer::Correct => 10.0,
            Answer::Incorrect(_) => -10.0,
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

    fn response_matches_known(&self, response: &str) -> Answer {
        match response == self.known {
            true => Answer::Correct,
            false => Answer::Incorrect(response.to_string()),
        }
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut game = Game::new().await;
    // init the words
    let mut words = init_words();
    let mut practiced_words = vec![];

    println!("Welcome to language learning!");

    loop {
        // Pop, give, process response, add to list, sort by
        if let Some(mut word) = words.pop() {
            word.display();
            let response_outcome = word.response_matches_known(&read_string());
            word.update_rating(&response_outcome);
            game.resolve_answer(&word, &response_outcome).await;
            practiced_words.push(word);
        } else {
            // practiced all the words, remake the list
            words = practiced_words;
            words.reverse(); // popping and then pushing reverses the order we started with
            words.sort_by(|v, w| w.rating.total_cmp(&v.rating));
            practiced_words = vec![];
        }
    }
}
