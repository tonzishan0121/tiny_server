use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr::{self, NonNull};

use crate::database::{DatabaseConnection, DatabaseConnector, DatabaseRow};

#[allow(non_camel_case_types)]
enum sqlite3 {}

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open(filename: *const c_char, db: *mut *mut sqlite3) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        callback: Option<
            unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
        >,
        first_arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(ptr: *mut c_void);
}

/// A small owning wrapper around one SQLite database handle.
pub struct SqliteConnection {
    handle: NonNull<sqlite3>,
}

unsafe impl Send for SqliteConnection {}

/// Opens SQLite database connections.
#[derive(Clone, Copy, Default)]
pub struct SqliteConnector;

impl DatabaseConnector for SqliteConnector {
    type Connection = SqliteConnection;

    fn open(&self, path: impl AsRef<Path>) -> Result<Self::Connection, String> {
        SqliteConnection::open(path)
    }

    fn open_memory(&self) -> Result<Self::Connection, String> {
        SqliteConnection::open_memory()
    }
}

impl SqliteConnection {
    /// Opens a SQLite database file, creating it when needed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_string_lossy();
        Self::open_raw(&path)
    }

    /// Opens a private in-memory SQLite database.
    pub fn open_memory() -> Result<Self, String> {
        Self::open_raw(":memory:")
    }

    fn open_raw(path: &str) -> Result<Self, String> {
        let path =
            CString::new(path).map_err(|_| "database path contains a nul byte".to_string())?;
        let mut handle = ptr::null_mut();
        let code = unsafe { sqlite3_open(path.as_ptr(), &mut handle) };
        let Some(handle) = NonNull::new(handle) else {
            return Err(format!("failed to open sqlite database, code {code}"));
        };

        if code == 0 {
            let connection = Self { handle };
            connection.execute("PRAGMA foreign_keys = ON;")?;
            return Ok(connection);
        }

        let message = sqlite_error_message(handle.as_ptr());
        unsafe {
            sqlite3_close(handle.as_ptr());
        }
        Err(message)
    }

    fn last_error_message(&self) -> String {
        sqlite_error_message(self.handle.as_ptr())
    }
}

impl DatabaseConnection for SqliteConnection {
    fn execute(&self, sql: &str) -> Result<(), String> {
        let sql = CString::new(sql).map_err(|_| "sql contains a nul byte".to_string())?;
        let mut error_message = ptr::null_mut();
        let code = unsafe {
            sqlite3_exec(
                self.handle.as_ptr(),
                sql.as_ptr(),
                None,
                ptr::null_mut(),
                &mut error_message,
            )
        };

        if code == 0 {
            return Ok(());
        }

        Err(take_error_message(error_message).unwrap_or_else(|| self.last_error_message()))
    }

    fn query(&self, sql: &str) -> Result<Vec<DatabaseRow>, String> {
        let sql = CString::new(sql).map_err(|_| "sql contains a nul byte".to_string())?;
        let mut rows = Vec::<DatabaseRow>::new();
        let mut error_message = ptr::null_mut();
        let code = unsafe {
            sqlite3_exec(
                self.handle.as_ptr(),
                sql.as_ptr(),
                Some(collect_row),
                (&mut rows as *mut Vec<DatabaseRow>).cast(),
                &mut error_message,
            )
        };

        if code == 0 {
            return Ok(rows);
        }

        Err(take_error_message(error_message).unwrap_or_else(|| self.last_error_message()))
    }
}

unsafe extern "C" fn collect_row(
    rows: *mut c_void,
    column_count: c_int,
    values: *mut *mut c_char,
    _: *mut *mut c_char,
) -> c_int {
    let rows = unsafe { &mut *(rows as *mut Vec<DatabaseRow>) };
    let values = unsafe { std::slice::from_raw_parts(values, column_count as usize) };
    let row = values
        .iter()
        .map(|value| {
            if value.is_null() {
                return String::new();
            }

            unsafe { CStr::from_ptr(*value) }
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    rows.push(row);
    0
}

impl Drop for SqliteConnection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.handle.as_ptr());
        }
    }
}

fn sqlite_error_message(handle: *mut sqlite3) -> String {
    let message = unsafe { sqlite3_errmsg(handle) };
    if message.is_null() {
        return "sqlite error".to_string();
    }

    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

fn take_error_message(message: *mut c_char) -> Option<String> {
    if message.is_null() {
        return None;
    }

    let output = unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned();
    unsafe {
        sqlite3_free(message.cast());
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::database::{DatabaseConnection, DatabaseConnector};

    use super::SqliteConnector;

    #[test]
    fn opens_memory_database_and_executes_sql() {
        let connector = SqliteConnector;
        let connection = connector.open_memory().expect("should open in-memory db");

        connection
            .execute("CREATE TABLE rooms (id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
            .expect("should create table");
        connection
            .execute("INSERT INTO rooms (name) VALUES ('lobby');")
            .expect("should insert row");
        let rows = connection
            .query("SELECT name FROM rooms ORDER BY id;")
            .expect("should query rows");

        assert_eq!(rows, vec![vec!["lobby".to_string()]]);
    }

    #[test]
    fn opens_file_database() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tiny_server_sqlite_{unique}.db"));

        {
            let connector = SqliteConnector;
            let connection = connector.open(&path).expect("should open file db");
            connection
                .execute("CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
                .expect("should create table");
        }

        assert!(path.exists());
        fs::remove_file(path).expect("should remove temp db");
    }
}
