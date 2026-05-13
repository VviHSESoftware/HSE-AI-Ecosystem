pub mod schema;
pub mod models;

use sea_query::{
    Alias, ColumnDef, CommonTableExpression, Expr, ForeignKey, ForeignKeyAction, Index,
    JoinType, OnConflict, Order, PostgresQueryBuilder, Query, Table, UnionType, WithClause,
};
use sea_query_binder::SqlxBinder;
use sqlx::{PgPool, Row};

use self::schema::*;
use self::models::*;

#[derive(Clone)]
pub struct KbRepository {
    pub pool: PgPool,
}

impl KbRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn init_schema(&self) -> Result<(), sqlx::Error> {
        let stmts =[
            Table::create()
                .table(Modules::Table)
                .if_not_exists()
                .col(ColumnDef::new(Modules::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Modules::ParentId).integer())
                .col(ColumnDef::new(Modules::Name).text().not_null())
                .col(ColumnDef::new(Modules::Type).text().not_null())
                .col(ColumnDef::new(Modules::Url).text())
                .col(ColumnDef::new(Modules::ExternalId).text())
                .col(ColumnDef::new(Modules::ExternalType).text())
                .col(ColumnDef::new(Modules::Path).text())
                .foreign_key(
                    ForeignKey::create().name("fk-module-parent")
                        .from(Modules::Table, Modules::ParentId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(Users::Table)
                .if_not_exists()
                .col(ColumnDef::new(Users::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Users::Email).text().not_null().unique_key())
                .build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleAccess::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleAccess::ModuleId).integer().not_null())
                .col(ColumnDef::new(ModuleAccess::UserId).integer().not_null())
                .primary_key(Index::create().col(ModuleAccess::ModuleId).col(ModuleAccess::UserId))
                .foreign_key(
                    ForeignKey::create().name("fk-access-module")
                        .from(ModuleAccess::Table, ModuleAccess::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create().name("fk-access-user")
                        .from(ModuleAccess::Table, ModuleAccess::UserId)
                        .to(Users::Table, Users::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleText::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleText::ModuleId).integer().not_null().primary_key())
                .col(ColumnDef::new(ModuleText::Content).text().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-text-module")
                        .from(ModuleText::Table, ModuleText::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleVideo::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleVideo::ModuleId).integer().not_null().primary_key())
                .col(ColumnDef::new(ModuleVideo::Transcription).text().not_null())
                .col(ColumnDef::new(ModuleVideo::TotalTime).double().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-video-module")
                        .from(ModuleVideo::Table, ModuleVideo::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(VideoChunks::Table)
                .if_not_exists()
                .col(ColumnDef::new(VideoChunks::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(VideoChunks::ModuleId).integer())
                .col(ColumnDef::new(VideoChunks::StartTime).double().not_null())
                .col(ColumnDef::new(VideoChunks::EndTime).double().not_null())
                .col(ColumnDef::new(VideoChunks::Content).text().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-videochunk-module")
                        .from(VideoChunks::Table, VideoChunks::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleFile::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleFile::ModuleId).integer().not_null().primary_key())
                .col(ColumnDef::new(ModuleFile::Content).text().not_null())
                .col(ColumnDef::new(ModuleFile::Filename).text().not_null())
                .col(ColumnDef::new(ModuleFile::PageCount).integer().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-file-module")
                        .from(ModuleFile::Table, ModuleFile::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleFilePages::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleFilePages::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(ModuleFilePages::ModuleId).integer())
                .col(ColumnDef::new(ModuleFilePages::PageNumber).integer().not_null())
                .col(ColumnDef::new(ModuleFilePages::Content).text().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-filepage-module")
                        .from(ModuleFilePages::Table, ModuleFilePages::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(ModuleSystem::Table)
                .if_not_exists()
                .col(ColumnDef::new(ModuleSystem::ModuleId).integer().not_null().primary_key())
                .col(ColumnDef::new(ModuleSystem::TokenHash).text().not_null())
                .foreign_key(
                    ForeignKey::create().name("fk-system-module")
                        .from(ModuleSystem::Table, ModuleSystem::ModuleId)
                        .to(Modules::Table, Modules::Id).on_delete(ForeignKeyAction::Cascade)
                ).build(PostgresQueryBuilder),

            Table::create()
                .table(HashCache::Table)
                .if_not_exists()
                .col(ColumnDef::new(HashCache::Hash).text().not_null().primary_key())
                .col(ColumnDef::new(HashCache::Data).binary().not_null())
                .build(PostgresQueryBuilder),
        ];

        let mut tx = self.pool.begin().await?;
        for stmt in stmts {
            sqlx::query(&stmt).execute(&mut *tx).await?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub async fn get_user_allowed_roots(&self, email: &str) -> Result<Vec<i32>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(ModuleAccess::ModuleId)
            .from(ModuleAccess::Table)
            .join(
                JoinType::Join,
                Users::Table,
                Expr::col((ModuleAccess::Table, ModuleAccess::UserId)).equals((Users::Table, Users::Id)),
            )
            .and_where(Expr::col((Users::Table, Users::Email)).eq(email))
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get("module_id")).collect())
    }

    pub async fn get_module_path(&self, module_id: i32) -> Result<Option<String>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(Modules::Path)
            .from(Modules::Table)
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get::<Option<String>, _>("path").unwrap_or_default()))
    }

    pub async fn get_modules_info_by_ids(&self, ids: &[i32]) -> Result<Vec<ModuleInfo>, sqlx::Error> {
        if ids.is_empty() { return Ok(vec![]); }
        let (sql, values) = Query::select()
            .columns([Modules::Id, Modules::ExternalType, Modules::Name])
            .from(Modules::Table)
            .and_where(Expr::col(Modules::Id).is_in(ids.iter().copied()))
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;
        let res = rows.into_iter().map(|r| ModuleInfo {
            id: r.get("id"),
            external_type: r.get("external_type"),
            name: r.get("name"),
        }).collect();
        Ok(res)
    }

    pub async fn get_video_chunks(&self, module_id: i32, start: f64, end: f64) -> Result<Vec<VideoChunkData>, sqlx::Error> {
        let (sql, values) = Query::select()
            .columns([VideoChunks::StartTime, VideoChunks::Content])
            .from(VideoChunks::Table)
            .and_where(Expr::col(VideoChunks::ModuleId).eq(module_id))
            .and_where(Expr::col(VideoChunks::StartTime).gte(start))
            .and_where(Expr::col(VideoChunks::EndTime).lte(end))
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| VideoChunkData {
            start_time: r.get("start_time"),
            content: r.get("content"),
        }).collect())
    }

    pub async fn get_module_url(&self, module_id: i32) -> Result<Option<String>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(Modules::Url)
            .from(Modules::Table)
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get::<Option<String>, _>("url").unwrap_or_default()))
    }

    pub async fn get_document_page_content(&self, module_id: i32, page: i32) -> Result<Option<String>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(ModuleFilePages::Content)
            .from(ModuleFilePages::Table)
            .and_where(Expr::col(ModuleFilePages::ModuleId).eq(module_id))
            .and_where(Expr::col(ModuleFilePages::PageNumber).eq(page))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("content")))
    }

    pub async fn get_text_module_content(&self, module_id: i32) -> Result<Vec<String>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(ModuleText::Content)
            .from(ModuleText::Table)
            .and_where(Expr::col(ModuleText::ModuleId).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get("content")).collect())
    }

    pub async fn get_all_document_pages_content(&self, module_id: i32) -> Result<Vec<String>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(ModuleFilePages::Content)
            .from(ModuleFilePages::Table)
            .and_where(Expr::col(ModuleFilePages::ModuleId).eq(module_id))
            .order_by(ModuleFilePages::PageNumber, Order::Asc)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get("content")).collect())
    }

    pub async fn get_available_structure(&self, roots: &[i32]) -> Result<Vec<StructureRowData>, sqlx::Error> {
        if roots.is_empty() { return Ok(vec![]); }

        let base_select = Query::select()
            .columns([Modules::Id, Modules::Name, Modules::Type, Modules::ParentId, Modules::Path])
            .expr_as(Expr::col((ModuleFile::Table, ModuleFile::PageCount)), Alias::new("page_count"))
            .expr_as(Expr::col((ModuleVideo::Table, ModuleVideo::TotalTime)), Alias::new("total_time"))
            .from(Modules::Table)
            .left_join(ModuleFile::Table, Expr::col((Modules::Table, Modules::Id)).equals((ModuleFile::Table, ModuleFile::ModuleId)))
            .left_join(ModuleVideo::Table, Expr::col((Modules::Table, Modules::Id)).equals((ModuleVideo::Table, ModuleVideo::ModuleId)))
            .and_where(Expr::col((Modules::Table, Modules::Id)).is_in(roots.iter().copied()))
            .to_owned();

        let recursive_select = Query::select()
            .columns([
                (Modules::Table, Modules::Id), (Modules::Table, Modules::Name),
                (Modules::Table, Modules::Type), (Modules::Table, Modules::ParentId),
                (Modules::Table, Modules::Path),
            ])
            .expr_as(Expr::col((ModuleFile::Table, ModuleFile::PageCount)), Alias::new("page_count"))
            .expr_as(Expr::col((ModuleVideo::Table, ModuleVideo::TotalTime)), Alias::new("total_time"))
            .from(Modules::Table)
            .join(JoinType::Join, TreeCte::Table, Expr::col((Modules::Table, Modules::ParentId)).equals((TreeCte::Table, Modules::Id)))
            .left_join(ModuleFile::Table, Expr::col((Modules::Table, Modules::Id)).equals((ModuleFile::Table, ModuleFile::ModuleId)))
            .left_join(ModuleVideo::Table, Expr::col((Modules::Table, Modules::Id)).equals((ModuleVideo::Table, ModuleVideo::ModuleId)))
            .to_owned();

        let mut base_select_stmt = base_select.to_owned();
        let recursive_select_stmt = recursive_select.to_owned();

        base_select_stmt.union(UnionType::All, recursive_select_stmt);

        let mut cte = CommonTableExpression::new();
        cte.query(base_select_stmt)
            .table_name(TreeCte::Table);


        let mut with_clause = WithClause::new();
        with_clause.recursive(true);
        with_clause.cte(cte);

        let (sql, values) = Query::select()
            .expr(Expr::cust("DISTINCT *"))
            .from(TreeCte::Table)
            .to_owned()
            .with(with_clause)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_with(&sql, values).fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|r| StructureRowData {
            id: r.get("id"), name: r.get("name"), r#type: r.get("type"),
            parent_id: r.get("parent_id"), path: r.get("path"),
            page_count: r.get("page_count"), total_time: r.get("total_time"),
        }).collect())
    }

    pub async fn get_system_id_by_token(&self, token_hash: &str) -> Result<Option<i32>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(ModuleSystem::ModuleId)
            .from(ModuleSystem::Table)
            .and_where(Expr::col(ModuleSystem::TokenHash).eq(token_hash))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("module_id")))
    }

    pub async fn get_module_id_secure(&self, system_id: i32, ext_id: &str, ext_type: &str) -> Result<Option<i32>, sqlx::Error> {
        let prefix = format!("/{}%", system_id);
        let (sql, values) = Query::select()
            .column(Modules::Id)
            .from(Modules::Table)
            .and_where(Expr::col(Modules::ExternalId).eq(ext_id))
            .and_where(Expr::col(Modules::ExternalType).eq(ext_type))
            .and_where(Expr::col(Modules::Path).like(prefix))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("id")))
    }

    // --- Insert / Update / Cache / Transaction Queries --- //

    pub async fn persist_text_module(&self, module_id: i32, text: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::insert()
            .into_table(ModuleText::Table)
            .columns([ModuleText::ModuleId, ModuleText::Content])
            .values_panic([module_id.into(), text.into()])
            .on_conflict(
                OnConflict::column(ModuleText::ModuleId).update_column(ModuleText::Content).to_owned()
            ).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        let (sql, values) = Query::update()
            .table(Modules::Table)
            .values([(Modules::Type, "Text".into())])
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn persist_video_module(&self, module_id: i32, transcription: &str, total_time: f64, chunks: &[NewVideoChunk]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::insert()
            .into_table(ModuleVideo::Table)
            .columns([ModuleVideo::ModuleId, ModuleVideo::Transcription, ModuleVideo::TotalTime])
            .values_panic([module_id.into(), transcription.into(), total_time.into()])
            .on_conflict(
                OnConflict::column(ModuleVideo::ModuleId)
                    .update_columns([ModuleVideo::Transcription, ModuleVideo::TotalTime]).to_owned()
            ).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        let (sql, values) = Query::delete().from_table(VideoChunks::Table).and_where(Expr::col(VideoChunks::ModuleId).eq(module_id)).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        if !chunks.is_empty() {
            let mut insert = Query::insert();
            insert.into_table(VideoChunks::Table)
                .columns([VideoChunks::ModuleId, VideoChunks::StartTime, VideoChunks::EndTime, VideoChunks::Content]);
            for chunk in chunks {
                insert.values_panic([module_id.into(), chunk.start.into(), chunk.end.into(), chunk.content.clone().into()]);
            }
            let (sql, values) = insert.build_sqlx(PostgresQueryBuilder);
            sqlx::query_with(&sql, values).execute(&mut *tx).await?;
        }

        let (sql, values) = Query::update()
            .table(Modules::Table)
            .values([(Modules::Type, "Video".into())])
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn persist_file_module(&self, module_id: i32, filename: &str, content: &str, page_count: i32, pages: &[NewFilePage]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::insert()
            .into_table(ModuleFile::Table)
            .columns([ModuleFile::ModuleId, ModuleFile::Content, ModuleFile::Filename, ModuleFile::PageCount])
            .values_panic([module_id.into(), content.into(), filename.into(), page_count.into()])
            .on_conflict(
                OnConflict::column(ModuleFile::ModuleId)
                    .update_columns([ModuleFile::Content, ModuleFile::Filename, ModuleFile::PageCount]).to_owned()
            ).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        let (sql, values) = Query::delete().from_table(ModuleFilePages::Table).and_where(Expr::col(ModuleFilePages::ModuleId).eq(module_id)).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        if !pages.is_empty() {
            let mut insert = Query::insert();
            insert.into_table(ModuleFilePages::Table)
                .columns([ModuleFilePages::ModuleId, ModuleFilePages::PageNumber, ModuleFilePages::Content]);
            for page in pages {
                insert.values_panic([module_id.into(), page.page_number.into(), page.content.clone().into()]);
            }
            let (sql, values) = insert.build_sqlx(PostgresQueryBuilder);
            sqlx::query_with(&sql, values).execute(&mut *tx).await?;
        }

        let (sql, values) = Query::update()
            .table(Modules::Table)
            .values([(Modules::Type, "File".into())])
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_hash_cache(&self, hash: &str) -> Result<Option<Vec<u8>>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(HashCache::Data)
            .from(HashCache::Table)
            .and_where(Expr::col(HashCache::Hash).eq(hash))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.get("data")))
    }

    pub async fn set_hash_cache(&self, hash: &str, data: &[u8]) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::insert()
            .into_table(HashCache::Table)
            .columns([HashCache::Hash, HashCache::Data])
            .values_panic([hash.into(), data.into()])
            .on_conflict(
                OnConflict::column(HashCache::Hash).do_nothing().to_owned()
            ).build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn create_system(&self, name: &str, token_hash: &str) -> Result<i32, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::insert()
            .into_table(Modules::Table)
            .columns([Modules::Name, Modules::Type])
            .values_panic([name.into(), "System".into()])
            .returning_col(Modules::Id)
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_one(&mut *tx).await?;
        let system_id: i32 = row.get("id");

        let path = format!("/{}", system_id);

        let (sql, values) = Query::update()
            .table(Modules::Table)
            .values([(Modules::Path, path.into())])
            .and_where(Expr::col(Modules::Id).eq(system_id))
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        let (sql, values) = Query::insert()
            .into_table(ModuleSystem::Table)
            .columns([ModuleSystem::ModuleId, ModuleSystem::TokenHash])
            .values_panic([system_id.into(), token_hash.into()])
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(system_id)
    }

    pub async fn add_module_base(&self, name: &str, url: &str, external_id: &str, external_type: &str, parent_module_id: i32) -> Result<i32, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::select()
            .column(Modules::Path)
            .from(Modules::Table)
            .and_where(Expr::col(Modules::Id).eq(parent_module_id))
            .build_sqlx(PostgresQueryBuilder);

        let p_row = sqlx::query_with(&sql, values).fetch_one(&mut *tx).await?;
        let p_path: Option<String> = p_row.get("path");
        let p_path_str = p_path.unwrap_or_default();

        let (sql, values) = Query::insert()
            .into_table(Modules::Table)
            .columns([Modules::Name, Modules::Type, Modules::Url, Modules::ExternalId, Modules::ExternalType, Modules::ParentId])
            .values_panic([name.into(), "Base".into(), url.into(), external_id.into(), external_type.into(), parent_module_id.into()])
            .returning_col(Modules::Id)
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_with(&sql, values).fetch_one(&mut *tx).await?;
        let new_id: i32 = row.get("id");

        let new_path = format!("{}/{}", p_path_str, new_id);

        let (sql, values) = Query::update()
            .table(Modules::Table)
            .values([(Modules::Path, new_path.into())])
            .and_where(Expr::col(Modules::Id).eq(new_id))
            .build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(new_id)
    }

    pub async fn add_module_access(&self, module_id: i32, emails: &[String]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let (sql, values) = Query::delete().from_table(ModuleAccess::Table).and_where(Expr::col(ModuleAccess::ModuleId).eq(module_id)).build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values).execute(&mut *tx).await?;

        for email in emails {
            let (sql, values) = Query::insert()
                .into_table(Users::Table)
                .columns([Users::Email])
                .values_panic([email.into()])
                .on_conflict(
                    OnConflict::column(Users::Email).update_column(Users::Email).to_owned()
                )
                .returning_col(Users::Id)
                .build_sqlx(PostgresQueryBuilder);

            let u_row = sqlx::query_with(&sql, values).fetch_one(&mut *tx).await?;
            let user_id: i32 = u_row.get("id");

            let (sql, values) = Query::insert()
                .into_table(ModuleAccess::Table)
                .columns([ModuleAccess::ModuleId, ModuleAccess::UserId])
                .values_panic([module_id.into(), user_id.into()])
                .on_conflict(
                    OnConflict::columns([ModuleAccess::ModuleId, ModuleAccess::UserId]).do_nothing().to_owned()
                )
                .build_sqlx(PostgresQueryBuilder);
            sqlx::query_with(&sql, values).execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_module(&self, module_id: i32) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::delete()
            .from_table(Modules::Table)
            .and_where(Expr::col(Modules::Id).eq(module_id))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }
}