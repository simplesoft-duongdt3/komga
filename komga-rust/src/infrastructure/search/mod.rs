use tantivy::{
    schema::{Schema, STORED, TEXT, Field, Value},
    Index, IndexWriter, TantivyDocument, doc, Term,
    collector::TopDocs,
    query::QueryParser,
};
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

pub struct SearchIndex {
    index: Index,
    writer: Mutex<IndexWriter>,
    schema: Schema,
    id_field: Field,
    title_field: Field,
    author_field: Field,
    series_field: Field,
}

static SEARCH_INDEX: Lazy<Option<SearchIndex>> = Lazy::new(|| {
    None
});

impl SearchIndex {
    pub fn new(index_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut schema_builder = Schema::builder();
        
        let id_field = schema_builder.add_text_field("id", TEXT | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let author_field = schema_builder.add_text_field("author", TEXT);
        let series_field = schema_builder.add_text_field("series", TEXT);
        
        let schema = schema_builder.build();
        
        let index = if index_path.exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };
        
        let writer = index.writer(50_000_000)?;
        
        Ok(Self {
            index,
            writer: Mutex::new(writer),
            schema,
            id_field,
            title_field,
            author_field,
            series_field,
        })
    }

    pub fn index_book(&self, id: &str, title: &str, author: &str, series: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = self.writer.lock().unwrap();
        
        writer.add_document(doc!(
            self.id_field => id,
            self.title_field => title,
            self.author_field => author,
            self.series_field => series,
        ))?;
        
        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        
        let query_parser = QueryParser::for_index(&self.index, vec![self.title_field, self.author_field, self.series_field]);
        let query = query_parser.parse_query(query_str)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id) = retrieved_doc.get_first(self.id_field) {
                if let Some(text) = id.as_str() {
                    results.push(text.to_string());
                }
            }
        }
        
        Ok(results)
    }

    pub fn delete_book(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(Term::from_field_text(self.id_field, id));
        writer.commit()?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = self.writer.lock().unwrap();
        writer.delete_all_documents()?;
        writer.commit()?;
        Ok(())
    }
}