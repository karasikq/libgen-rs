use console::Style;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use libgen_api::{mirrors::{MirrorList, SearchMirror, DownloadMirror}, error::Error, search::{SearchIn, SearchBuilder}, book::Book};
use reqwest::Client;

lazy_static! {
    static ref RED_STYLE: Style = Style::new().red();
}

pub fn select_search_mirror(mirrors: &MirrorList) -> Result<SearchMirror, Error> {
    let mirror_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Search mirror")
        .default(0)
        .items(&mirrors.search_mirrors)
        .interact_opt()
        .unwrap();
    mirrors.get_search_mirror(mirror_selection.unwrap())
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
pub fn input_search_option() -> Result<SearchIn, Error> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Search option")
        .default(0)
        .items(&OPTIONS[..])
        .interact()
        .unwrap();

    SearchIn::try_from(selection)
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

pub fn select_download_mirror(mirrors: &MirrorList) -> Result<DownloadMirror, Error> {
    let mirror_selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Download mirror")
        .default(0)
        .items(&mirrors.download_mirrors)
        .interact_opt()
        .unwrap();
    mirrors.get_download_mirror(mirror_selection.unwrap())
}

pub async fn init() -> Result<(), Error> {
    let client = Client::new();
    let mirrors = MirrorList::new();
    let Ok(search_mirror) = select_search_mirror(&mirrors) else {
        return Err("You must select a mirror")?;
    };
    let books = loop {
        let request = input_search_request().expect("Empty request");
        let search_option = input_search_option().unwrap();
        let results = input_results_count().unwrap();
        let search_query = SearchBuilder::new(
            request,
            search_mirror.search_url.clone(),
            search_mirror.cover_url.clone(),
            search_mirror.json_search_url.clone(),
        )
        .max_results(results)
        .search_option(search_option)
        .build();
        println!("Search at {}... This may take a while", search_mirror);
        let search_result = search_query.search().await?;
        if search_result.is_empty() {
            println!("Books not found");
            continue;
        } else {
            break search_result;
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
        let Ok(download_mirror) = select_download_mirror(&mirrors) else {
            return Err("You must select a mirror")?;
        };

        let pb = ProgressBar::new(0);
        pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
        pb.set_message("Downloading...");

        let _ = selected_book
            .download_to_path(
                Some(&client),
                download_mirror,
                dirs::download_dir().unwrap().to_str().unwrap(),
                Some(|downloaded, size| {
                    pb.set_length(size);
                    pb.set_position(downloaded);
                }),
            )
            .await;
        break;
    }

    Ok(())
}
