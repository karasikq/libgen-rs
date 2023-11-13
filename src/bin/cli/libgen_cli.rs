use console::Style;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use reqwest::Client;
use std::{fs::File, io::Write};

use libgen::api::{
    book::Book,
    mirrors::{Mirror, MirrorList, MirrorType},
    search::{Search, SearchOption},
};

lazy_static! {
    static ref RED_STYLE: Style = Style::new().red();
}

pub fn select_search_mirror(mirrors: &MirrorList) -> Result<Mirror, &'static str> {
    let mirror_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Search mirror")
        .default(0)
        .items(&mirrors.search_mirrors)
        .interact_opt()
        .unwrap();
    mirrors.get(MirrorType::Search, mirror_selection.unwrap())
}

pub fn input_search_request() -> Result<String, &'static str> {
    Ok(Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Search request")
        .interact_text()
        .expect("You must specify a request"))
}

const OPTIONS: &[&str; 11] = &[
    "Default",
    "Title",
    "Author",
    "Series",
    "Publisher",
    "Year",
    "Identifier",
    "Language",
    "MD5",
    "Tags",
    "Extension",
];
pub fn input_search_option() -> Result<SearchOption, &'static str> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Search option")
        .default(0)
        .items(&OPTIONS[..])
        .interact()
        .unwrap();

    SearchOption::try_from(selection)
}

pub fn input_results_count() -> Result<u32, &'static str> {
    let selections = &[25, 50, 100];

    Ok(selections[Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Results per request")
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap()])
}

pub fn fuzzyselect_book(books: &[Book]) -> Result<Book, &'static str> {
    let book = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select book")
        .default(0)
        .items(books)
        .interact_opt()
        .unwrap();
    Ok(books.get(book.expect("Book not selected")).unwrap().clone())
}

pub fn print_book_info(book: &Book) -> Result<(), &'static str> {
    println!("{}: {}", RED_STYLE.apply_to("ID"), book.id);
    println!("{}: {}", RED_STYLE.apply_to("Title"), book.title);
    println!("{}: {}", RED_STYLE.apply_to("Author"), book.author);
    println!(
        "{}: {:.2} Mb",
        RED_STYLE.apply_to("Filesize"),
        book.filesize.parse::<u32>().unwrap() as f32 / 1048576.0
    );
    println!("{}: {}", RED_STYLE.apply_to("Year"), book.year);
    println!("{}: {}", RED_STYLE.apply_to("Language"), book.language);
    println!("{}: {}", RED_STYLE.apply_to("Pages"), book.pages);
    println!("{}: {}", RED_STYLE.apply_to("Publisher"), book.publisher);
    println!("{}: {}", RED_STYLE.apply_to("Edition"), book.edition);
    println!("{}: {}", RED_STYLE.apply_to("MD5"), book.md5);
    println!("{}: {}", RED_STYLE.apply_to("Cover"), book.coverurl);
    Ok(())
}

pub fn select_download_mirror(mirrors: &MirrorList) -> Result<Mirror, &'static str> {
    let mirror_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Download mirror")
        .default(0)
        .items(&mirrors.download_mirrors)
        .interact_opt()
        .unwrap();
    mirrors.get(MirrorType::Download, mirror_selection.unwrap())
}

pub async fn init() -> Result<(), &'static str> {
    let client = Client::new();
    let mirrors = MirrorList::parse("libgen-rs/mirrors.json");
    let Ok(search_mirror) = select_search_mirror(&mirrors) else {
        return Err("You must select a mirror");
    };
    let books = loop {
        let request = input_search_request().expect("Empty request");
        let search_option = input_search_option().unwrap();
        let results = input_results_count().unwrap();
        let search_options = Search {
            mirror: search_mirror.clone(),
            request,
            results,
            search_option,
        };
        println!("Search at {}... This may take a while", search_mirror);
        let received_books = search_options.search(&client).await?;
        if received_books.is_empty() {
            println!("Books not found");
            continue;
        } else {
            break received_books;
        }
    };
    loop {
        let selected_book = fuzzyselect_book(&books).expect("Empty book");
        print_book_info(&selected_book).unwrap();
        if !Confirm::new()
            .with_prompt("Do you want to download this book?")
            .interact()
            .unwrap()
        {
            continue;
        }
        let download_request = select_download_mirror(&mirrors).unwrap();
        let down_req = download_request
            .download_book(&client, &selected_book)
            .await?;
        let total_size = down_req.content_length().unwrap();

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
        pb.set_message("Downloading...");

        let mut book_download_path = dirs::download_dir().unwrap();
        book_download_path.push("libgen-rs");
        std::fs::create_dir_all(&book_download_path).unwrap();

        let len = selected_book.title.len().min(249);
        book_download_path.push(&selected_book.title[0..len]);
        book_download_path.set_extension(&selected_book.extension);
        let mut stream = down_req.bytes_stream();
        let mut file = File::create(book_download_path).unwrap();
        let mut downloaded: u64 = 0;
        while let Some(item) = stream.next().await {
            let chunk = item.or(Err("Error while downloading file")).unwrap();
            file.write_all(&chunk).unwrap();
            let new = (downloaded + (chunk.len() as u64)).min(total_size);
            downloaded = new;
            pb.set_position(new);
        }
        break;
    }

    Ok(())
}
