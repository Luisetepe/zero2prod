use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings, Settings};
use zero2prod::startup::{get_connection_pool, Application};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestContext {
    pub server_address: String,
    pub db_pool: PgPool,

    settings: Settings,
}

impl TestContext {
    pub async fn create_stub_app() -> Self {
        Lazy::force(&TRACING);

        // Randomise configuration to ensure test isolation
        let configuration = {
            let mut c = get_configuration().expect("Failed to read configuration.");
            // Use a different database for each test case
            c.database.database_name = Uuid::new_v4().to_string();
            // Use a random OS port
            c.application.port = 0;
            c
        };

        configure_database(&configuration.database).await;

        let application = Application::build(configuration.clone())
            .await
            .expect("Failed to build application.");

        // Get the port before spawning the application
        let address = format!("http://127.0.0.1:{}", application.port());
        tokio::spawn(application.run_until_stopped());

        TestContext {
            server_address: address,
            db_pool: get_connection_pool(&configuration.database),
            settings: configuration,
        }
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.server_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn cleanup(&self) {
        self.db_pool.close().await;

        // Delete database
        let mut connection = PgConnection::connect_with(&self.settings.database.without_db())
            .await
            .expect("Failed to connect to Postgres");
        connection
            .execute(
                format!(
                    r#"DROP DATABASE "{}" WITH ( FORCE );"#,
                    self.settings.database.database_name
                )
                .as_str(),
            )
            .await
            .expect("Failed to delete database.");

        connection.close().await.unwrap();
    }
}

async fn configure_database(config: &DatabaseSettings) {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    connection.close().await.unwrap();

    // Migrate database
    let mut connection = PgConnection::connect_with(&config.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&mut connection)
        .await
        .expect("Failed to migrate the database");

    connection.close().await.unwrap();
}
