use crate::tools::xml::{ToXml, XmlBuilder};

#[derive(Clone, Debug)]
pub struct FileBlock {
    pub path: String,
    pub imports: Vec<String>,
    pub usages: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ImportUsageOutput {
    pub files: Vec<FileBlock>,
    pub next_page: Option<u64>,
    pub system_message: String,
}

impl ToXml for ImportUsageOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();
        builder.start_element("ToolResponse")?;
        for f in &self.files {
            builder.start_element("file")?;
            builder.write_element("path", &f.path)?;

            let mut imports_text = String::new();
            for line in &f.imports {
                imports_text.push_str(line);
                if !line.ends_with('\n') {
                    imports_text.push('\n');
                }
            }
            builder.write_cdata_element("imports", &imports_text)?;

            let mut usages_text = String::new();
            for line in &f.usages {
                usages_text.push_str(line);
                if !line.ends_with('\n') {
                    usages_text.push('\n');
                }
            }
            builder.write_cdata_element("usages", &usages_text)?;

            builder.end_element("file")?;
        }
        builder.write_optional_numeric_element("next-page", &self.next_page)?;
        builder.write_cdata_element("system-message", &self.system_message)?;
        builder.end_element("ToolResponse")?;
        builder.finish()
    }
}
