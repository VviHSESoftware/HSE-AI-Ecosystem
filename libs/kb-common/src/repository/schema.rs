use sea_query::Iden;

#[derive(Iden)]
pub enum Modules { Table, Id, ParentId, Name, Type, Url, ExternalId, ExternalType, Path }

#[derive(Iden)]
pub enum Users { Table, Id, Email }

#[derive(Iden)]
pub enum ModuleAccess { Table, ModuleId, UserId }

#[derive(Iden)]
pub enum ModuleText { Table, ModuleId, Content }

#[derive(Iden)]
pub enum ModuleVideo { Table, ModuleId, Transcription, TotalTime }

#[derive(Iden)]
pub enum VideoChunks { Table, Id, ModuleId, StartTime, EndTime, Content }

#[derive(Iden)]
pub enum ModuleFile { Table, ModuleId, Content, Filename, PageCount }

#[derive(Iden)]
pub enum ModuleFilePages { Table, Id, ModuleId, PageNumber, Content }

#[derive(Iden)]
pub enum ModuleSystem { Table, ModuleId, TokenHash }

#[derive(Iden)]
pub enum HashCache { Table, Hash, Data }

#[derive(Iden)]
pub enum TreeCte { Table }