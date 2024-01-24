use std::{
    convert::TryFrom,
    error::Error,
    fmt::Display,
    fs::{read_to_string, write, File},
    io::{stdin, stdout, Stdin, Write},
    path::Path,
    sync::OnceLock,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Глобальный контейнер для решения зависимостей объектов
pub static CONTAINER: OnceLock<Container> = OnceLock::new();

/// Главная функция программы
fn main() -> Result<()> {
    CONTAINER.get_or_init(|| Container::default());

    App::run()
}

/// Приложение
pub struct App;

impl App {
    pub fn run() -> Result<()> {
        let res = || -> Result<()> {
            HelloModel.exec()?;
            AddEntryModel.exec()?;
            ViewListEntryModel.exec()?;

            Ok(())
        }();

        if let Err(err) = res.as_ref() {
            if err.downcast_ref::<AppError>().is_some() {
                println!("Ошибка: {}", err);
                return Ok(());
            }
        }

        res
    }
}

/// Ошибки приложения
#[derive(Debug)]
pub enum AppError {
    Exit,
    Msg(&'static str),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Exit => "Выход",
                Self::Msg(s) => s,
            }
        )
    }
}

impl Error for AppError {}

/// Интерфейс для моделей
pub trait ModelTrait: Default {
    fn exec(&self) -> Result<()>;
}

/// Модели

/// Модель приветствия
#[derive(Default)]
pub struct HelloModel;

impl ModelTrait for HelloModel {
    fn exec(&self) -> Result<()> {
        println!("Привет!");

        Ok(())
    }
}

/// Модель добавления новой записи в планер
#[derive(Default)]
pub struct AddEntryModel;

impl ModelTrait for AddEntryModel {
    fn exec(&self) -> Result<()> {
        match || -> Result<()> {
            loop {
                let entry = Entry::try_from(&stdin())?;
                entry.save()?;
            }
        }() {
            Err(e) if matches!(e.downcast_ref(), Some(&AppError::Exit)) => Ok(()),
            res => res,
        }
    }
}

/// Модель отображения записей в планере
#[derive(Default)]
pub struct ViewListEntryModel;

impl ModelTrait for ViewListEntryModel {
    fn exec(&self) -> Result<()> {
        let view = CONTAINER.get().unwrap().list_view();
        println!("{}", view);

        Ok(())
    }
}

/// Интрефейс записи для планера
pub trait EntryTrait: Display {
    fn time(&self) -> &str;

    fn target(&self) -> &str;
}

/// Запись для планера
#[derive(Default, Debug, Clone)]
pub struct Entry {
    time: String,
    target: String,
}

/// Реализация интерфейса записи для планера
impl EntryTrait for Entry {
    fn time(&self) -> &str {
        &self.time
    }

    fn target(&self) -> &str {
        &self.target
    }
}

/// Создание записи планера из консольного ввода пользователя
impl TryFrom<&Stdin> for Entry {
    type Error = Box<dyn Error>;

    fn try_from(stdin: &Stdin) -> std::result::Result<Self, Self::Error> {
        let mut entry = Self::default();

        print!("Что планируешь делать?: ");
        stdout().flush()?;
        stdin.read_line(&mut entry.target)?;
        entry.target = entry.target.trim().to_owned();

        if entry.target.is_empty() {
            Err(AppError::Exit)?
        }

        loop {
            print!("Во сколько (пример 9:30): ");
            stdout().flush()?;
            entry.time = String::new();
            stdin.read_line(&mut entry.time)?;

            if entry.time.trim().is_empty() {
                Err(AppError::Exit)?
            }

            entry.time = entry
                .time
                .chars()
                .filter(|c| matches!(c, '0'..='9' | ':'))
                .collect::<String>();

            match || -> Result<()> {
                match entry.time.split_once(':') {
                    Some((hours, mins)) => {
                        let hours: i8 = hours.parse()?;
                        let mins: i8 = mins.parse()?;

                        if !(0..=23).contains(&hours) || !(0..=59).contains(&mins) {
                            Err(AppError::Msg("Неверное время."))?;
                        }

                        entry.time = format!("{}:{:0>2}", hours, mins);
                        Ok(())
                    }
                    None => Err(AppError::Msg("Неверное время."))?,
                }
            }() {
                Ok(..) => break,
                Err(e) => eprintln!("Ошибка: {}", e),
            }
        }

        Ok(entry)
    }
}

/// Получение объекта интерфейса записи для планера
impl From<Entry> for Box<dyn EntryTrait> {
    fn from(val: Entry) -> Self {
        Box::new(val)
    }
}

/// Отображение записи планера
impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Время: {}\nЗадача: {}", self.time, self.target)
    }
}

/// Сохранение записи планера
impl Entry {
    pub fn save(&self) -> Result<()> {
        let storage = CONTAINER.get().unwrap().storage();
        storage.save(self.clone().into())?;

        Ok(())
    }
}

/// Представление списка записей
#[derive(Default)]
pub struct ListView;

impl Display for ListView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let list = CONTAINER
            .get()
            .unwrap()
            .storage()
            .read()
            .expect("Не удалось прочитать файл.");

        let output = list
            .iter()
            .map(|entry| entry.to_string())
            .collect::<Vec<String>>()
            .join("--------------------------\n");

        writeln!(f, "====================================")?;
        writeln!(f, "Мое расписание:\n\n{}", output)?;
        writeln!(f, "====================================")?;

        Ok(())
    }
}

/// Контейнер для разрешения зависимостей
pub struct Container {
    storage: Storage,
    list_view: ListView,
}

/// Создание контейнера
impl Default for Container {
    fn default() -> Self {
        let storage = Storage::new("./my-planner.txt");
        let list_view = ListView::default();

        Self { storage, list_view }
    }
}

/// Получение объектов из контейнера
impl Container {
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn list_view(&self) -> &ListView {
        &self.list_view
    }
}

/// Хранилище записей
pub struct Storage {
    path: String,
}

impl Storage {
    /// Создание хранилища с указанием пути к файлу хранилища
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Добавление и сохранение отсортированных записей планера в файл
    pub fn save(&self, entry: Box<dyn EntryTrait>) -> Result<()> {
        let mut list = self.read()?;
        list.push(entry);
        list.sort();

        let mut file = File::create(&self.path)?;
        for entry in list {
            file.write_fmt(format_args!("{}\n{}\n\n", entry.time(), entry.target()))?;
        }

        file.flush()?;

        println!("Сохранено");
        println!("====================================");

        Ok(())
    }

    /// Получение списка записей планера из файла
    pub fn read(&self) -> Result<Vec<Box<dyn EntryTrait>>> {
        if !Path::new(&self.path).exists() {
            write(&self.path, "")?;
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        let buf = read_to_string(&self.path)?;
        for block in buf.split_terminator("\n\n").collect::<Vec<&str>>() {
            let entry = block.trim().split_once('\n').map(|(time, target)| Entry {
                time: time.to_owned(),
                target: target.to_owned(),
            });
            if let Some(entry) = entry {
                list.push(entry.into());
            }
        }

        Ok(list)
    }
}

/// Реализация сортировки записей

impl Ord for Box<dyn EntryTrait> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a: i32 = self.time().replace(':', "").parse().unwrap();
        let b: i32 = other.time().replace(':', "").parse().unwrap();
        a.cmp(&b)
    }
}

impl Eq for Box<dyn EntryTrait> {}

impl PartialEq for Box<dyn EntryTrait> {
    fn eq(&self, other: &Self) -> bool {
        self.time() == other.time() && self.target() == other.target()
    }
}

impl PartialOrd for Box<dyn EntryTrait> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
