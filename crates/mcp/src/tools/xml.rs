use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use regex::Regex;
use std::io::Cursor;

/// Removes CDATA sections from XML, replacing them with their inner content
/// This is useful when the XML is intended for LLM consumption rather than parsing
pub fn remove_cdata_sections(xml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cdata_regex = Regex::new(r"(?s)<!\[CDATA\[(.*?)\]\]>")?;

    // Replace all CDATA sections with their inner content
    let result = cdata_regex.replace_all(xml, "$1");

    Ok(result.to_string())
}

/// Trait for converting tool output structures to XML
pub trait ToXml {
    /// Convert the structure to XML string with proper formatting
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>>;

    /// Convert the structure to XML string with CDATA sections removed.
    /// This is useful when the XML is intended for LLM consumption rather than parsing.
    fn to_xml_without_cdata(&self) -> Result<String, Box<dyn std::error::Error>> {
        let xml = self.to_xml()?;
        remove_cdata_sections(&xml)
    }
}

pub struct XmlBuilder {
    writer: Writer<Cursor<Vec<u8>>>,
}

impl XmlBuilder {
    pub fn new() -> Self {
        let writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        Self { writer }
    }

    /// Start a new XML element
    pub fn start_element(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let elem = BytesStart::new(name);
        self.writer.write_event(Event::Start(elem))?;
        Ok(())
    }

    /// End the current XML element
    pub fn end_element(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let elem = BytesEnd::new(name);
        self.writer.write_event(Event::End(elem))?;
        Ok(())
    }

    pub fn write_element(
        &mut self,
        name: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_element(name)?;
        let text = BytesText::new(content);
        self.writer.write_event(Event::Text(text))?;
        self.end_element(name)?;
        Ok(())
    }

    pub fn write_cdata_element(
        &mut self,
        name: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_element(name)?;

        // Only add newlines if content doesn't already start/end with them
        let formatted_content = if content.starts_with('\n') && content.ends_with('\n') {
            content.to_string()
        } else if content.starts_with('\n') {
            format!("{content}\n")
        } else if content.ends_with('\n') {
            format!("\n{content}")
        } else {
            format!("\n{content}\n")
        };

        let cdata = Event::CData(BytesCData::new(formatted_content));
        self.writer.write_event(cdata)?;
        self.end_element(name)?;
        Ok(())
    }

    pub fn write_optional_element(
        &mut self,
        name: &str,
        content: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(content) = content {
            self.write_element(name, content)?;
        }
        Ok(())
    }

    pub fn write_optional_cdata_element(
        &mut self,
        name: &str,
        content: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(content) = content {
            self.write_cdata_element(name, content)?;
        }
        Ok(())
    }

    pub fn write_numeric_element<T: std::fmt::Display>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_element(name, &value.to_string())
    }

    pub fn write_optional_numeric_element<T: std::fmt::Display>(
        &mut self,
        name: &str,
        value: &Option<T>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(value) = value {
            self.write_numeric_element(name, value)?;
        }
        Ok(())
    }

    pub fn write_boolean_element(
        &mut self,
        name: &str,
        value: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_element(name, if value { "true" } else { "false" })
    }

    pub fn finish(self) -> Result<String, Box<dyn std::error::Error>> {
        let inner = self.writer.into_inner();
        let bytes = inner.into_inner();
        Ok(String::from_utf8(bytes)?)
    }
}

impl Default for XmlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_cdata_sections() {
        let xml_with_cdata = r#"<root>
    <element><![CDATA[
        Some code content
        with special characters <>&"'
    ]]></element>
    <another><![CDATA[More content]]></another>
    <normal>Regular content</normal>
</root>"#;

        let expected = r#"<root>
    <element>
        Some code content
        with special characters <>&"'
    </element>
    <another>More content</another>
    <normal>Regular content</normal>
</root>"#;

        let result = remove_cdata_sections(xml_with_cdata).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_remove_cdata_sections_empty_cdata() {
        let xml_with_empty_cdata = r#"<element><![CDATA[]]></element>"#;
        let expected = r#"<element></element>"#;

        let result = remove_cdata_sections(xml_with_empty_cdata).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_remove_cdata_sections_no_cdata() {
        let xml_without_cdata = r#"<root><element>Normal content</element></root>"#;

        let result = remove_cdata_sections(xml_without_cdata).unwrap();
        assert_eq!(result, xml_without_cdata);
    }

    #[test]
    fn test_remove_cdata_sections_multiple_on_same_line() {
        let xml = r#"<root><a><![CDATA[content1]]></a><b><![CDATA[content2]]></b></root>"#;
        let expected = r#"<root><a>content1</a><b>content2</b></root>"#;

        let result = remove_cdata_sections(xml).unwrap();
        assert_eq!(result, expected);
    }
}
