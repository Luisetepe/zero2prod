use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, Settings};
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

        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{}", port);

        let mut config = get_configuration().expect("Failed to read configuration.");
        config.database.database_name = Uuid::new_v4().to_string();

        // Create database
        let mut connection = PgConnection::connect_with(&config.database.without_db())
            .await
            .expect("Failed to connect to Postgres");
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.database.database_name).as_str())
            .await
            .expect("Failed to create database.");
        #[allow(clippy::let_underscore_future)]
        let _ = connection.close().await;

        // Migrate database
        let connection_pool = PgPool::connect_with(config.database.with_db())
            .await
            .expect("Failed to connect to Postgres.");
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database");

        let server =
            zero2prod::run(listener, connection_pool.clone()).expect("Failed to bind address");

        #[allow(clippy::let_underscore_future)]
        let _ = tokio::spawn(server);

        TestContext {
            server_address: address,
            db_pool: connection_pool,
            settings: config,
        }
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
                    r#"DROP DATABASE "{}";"#,
                    self.settings.database.database_name
                )
                .as_str(),
            )
            .await
            .expect("Failed to delete database.");
    }
}
