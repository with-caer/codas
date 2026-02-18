//! Coda Markdown parser.
//!
//! # Unstable
//!
//! The APIs exposed by this module are _primarily_
//! for use by automated tooling (macros, CLIs, etc.);
//! the exact APIs are subject to change, and may
//! not be well-optimized.

use core::{iter::Peekable, ops::Range};

use logos::{Lexer, Logos};
use snafu::Snafu;
use token::Token;

use crate::types::{Coda, DataField, DataType, Text, Type};

mod token;

/// Parses `markdown` into a [`Coda`].
pub fn parse(markdown: &str) -> Result<Coda, ParseError> {
    // Parse the raw coda from the markdown.
    let markdown = markdown.trim();
    let mut parser = Parser::new(markdown);
    let parsed_coda = parser.parse()?;

    // Prepare an in-memory coda.
    let docs = if parsed_coda.docs.is_empty() {
        None
    } else {
        Some(markdown[parsed_coda.docs].trim().into())
    };
    let mut coda = Coda::new(parsed_coda.global_name, parsed_coda.local_name, docs, &[]);

    // Create data types.
    for (ordinal, parsed_data) in parsed_coda.data.into_iter().enumerate() {
        // Ordinals are 1-indexed.
        let ordinal = (ordinal + 1) as u16;

        // Extract docs.
        let docs = if parsed_data.docs.is_empty() {
            None
        } else {
            Some(markdown[parsed_data.docs].trim().into())
        };

        // Extract fields.
        let mut data = DataType::new(parsed_data.name, docs, ordinal, &[], &[]);
        for parsed_field in parsed_data.fields {
            // Extract docs.
            let docs = if parsed_field.docs.is_empty() {
                None
            } else {
                Some(markdown[parsed_field.docs].trim().into())
            };

            // Shorthand type resolver.
            let resolve_typing = |typing: Text| match coda.type_from_name(&typing) {
                Some(typing) => typing,
                None => Type::Data(DataType::new_fluid(typing, None)),
            };

            // Extract typing.
            let typing = match parsed_field.typing {
                ParsedFieldType::Scalar(typing) => resolve_typing(typing),
                ParsedFieldType::List(dimensions, typing) => {
                    let mut typing = resolve_typing(typing);
                    for _ in 0..dimensions {
                        typing = Type::List(typing.into());
                    }
                    typing
                }
                ParsedFieldType::Map(key_typing, value_typing) => {
                    Type::Map((resolve_typing(key_typing), resolve_typing(value_typing)).into())
                }
            };

            data = data.with(DataField {
                name: parsed_field.name,
                docs,
                typing,
                optional: parsed_field.optional,
                flattened: parsed_field.flattened,
            });
        }

        coda.data.push(data);
    }

    Ok(coda)
}

/// A Markdown parser for codas.
struct Parser<'lexer> {
    /// The token lexer being parsed.
    lexer: Peekable<Lexer<'lexer, Token<'lexer>>>,
}

impl<'lexer> Parser<'lexer> {
    /// Creates a new parser for `text`.
    fn new(text: &'lexer str) -> Self {
        Self {
            lexer: Token::lexer(text).peekable(),
        }
    }

    /// Parses the next [`Coda`] from the text.
    fn parse(&mut self) -> Result<ParsedCoda, ParseError> {
        Ok(self.take_coda()?.unwrap())
    }

    /// Takes the next [`Token::Coda`].
    fn take_coda(&mut self) -> Result<Option<ParsedCoda>, ParseError> {
        let name = match self.lexer.next() {
            Some(Ok(Token::Coda(name))) => name,
            _ => return Err(ParseError::ExpectedCoda),
        };

        let mut coda = ParsedCoda {
            global_name: name.0.into(),
            local_name: name.1.into(),
            docs: 0..0,
            data: alloc::vec![],
        };

        // Parse docs.
        let (docs, whitespace) = self.take_docs_lines()?;
        assert!(docs.is_empty() || whitespace == 0);
        if !docs.is_empty() && whitespace != 0 {
            return Err(ParseError::UnexpectedDocsIndentation { actual: whitespace });
        }
        coda.docs = docs;

        // Parse data types.
        while let Some(data_type) = self.take_data()? {
            coda.data.push(data_type);
        }

        Ok(Some(coda))
    }

    /// Takes the next [`Token::Data`].
    fn take_data(&mut self) -> Result<Option<ParsedDataType>, ParseError> {
        let name = match self.lexer.peek() {
            Some(Ok(Token::Data(name))) => {
                let name = (*name).into();
                self.lexer.next();
                name
            }
            None | Some(Ok(..)) => return Ok(None),
            _ => return Err(ParseError::ExpectedDataType),
        };

        let mut data_type = ParsedDataType {
            name,
            docs: 0..0,
            fields: alloc::vec![],
        };

        // Parse the data's docs.
        let (docs, whitespace) = self.take_docs_lines()?;
        if !docs.is_empty() && whitespace != 0 {
            return Err(ParseError::UnexpectedDocsIndentation { actual: whitespace });
        }
        data_type.docs = docs;

        // Parse the data's fields.
        while let Some(data_field) = self.take_data_field()? {
            data_type.fields.push(data_field);
        }

        Ok(Some(data_type))
    }

    /// Takes the next [`Token::DataField`].
    fn take_data_field(&mut self) -> Result<Option<ParsedField>, ParseError> {
        let mut field = match self.lexer.peek() {
            Some(Ok(Token::DataField(field))) => {
                let field = field.clone();
                self.lexer.next();
                field
            }
            None | Some(Ok(..)) => return Ok(None),
            _ => return Err(ParseError::ExpectedDataField),
        };

        // Parse the fields' docs.
        let (docs, whitespace) = self.take_docs_lines()?;
        if !docs.is_empty() && whitespace == 0 {
            return Err(ParseError::ExpectedDocsIndentation {
                minimum_expected: 1,
            });
        }
        field.docs = docs;

        Ok(Some(field))
    }

    /// Takes the next contiguous set of [`Token::DocsLine`]s
    /// with the same level of leading whitespace.
    fn take_docs_lines(&mut self) -> Result<(Range<usize>, usize), ParseError> {
        let mut leading_whitespace = 0;
        let mut range = 0..0;

        while let Some(token) = self.lexer.peek() {
            match token {
                Ok(Token::DocsLine((line, line_range, line_whitespace))) => {
                    // Init.
                    if range.is_empty() {
                        range = line_range.clone();
                        leading_whitespace = *line_whitespace;
                        self.lexer.next();
                        continue;
                    }

                    // Iter.
                    if line == &"\n" || line == &"\r" || *line_whitespace >= leading_whitespace {
                        range.end = line_range.end;
                        self.lexer.next();
                        continue;
                    }

                    // Done.
                    break;
                }

                Ok(..) => break,

                _ => return Err(ParseError::UnexpectedError),
            }
        }

        Ok((range, leading_whitespace))
    }
}

/// [`Coda`] parsed from text.
#[derive(Clone, Debug, PartialEq)]
struct ParsedCoda {
    global_name: Text,
    local_name: Text,
    docs: Range<usize>,
    data: alloc::vec::Vec<ParsedDataType>,
}

/// [`DataType`] parsed from text.
#[derive(Clone, Debug, PartialEq)]
struct ParsedDataType {
    name: Text,
    docs: Range<usize>,
    fields: alloc::vec::Vec<ParsedField>,
}

/// [`DataField`] parsed from text.
#[derive(Clone, Debug, PartialEq)]
struct ParsedField {
    name: Text,

    /// The span of the lexer's contents
    /// containing the field's docs.
    docs: Range<usize>,

    /// The parsed (but unresolved) typing.
    typing: ParsedFieldType,

    /// True if the field is optional.
    optional: bool,

    /// True if the field is flattened.
    flattened: bool,
}

/// Unresolved typing of a [`ParsedField`].
#[derive(Clone, Debug, PartialEq)]
enum ParsedFieldType {
    /// A single value of one type.
    Scalar(Text),

    /// An N-dimensional list of values of one type.
    List(usize, Text),

    /// A mapping of one type to another.
    Map(Text, Text),
}

/// Enumeration of errors that may occur when parsing codas.
#[derive(Debug, Snafu)]
pub enum ParseError {
    #[snafu(display("Expected to parse a Coda header."))]
    ExpectedCoda,

    #[snafu(display("Expected to parse a Data type header."))]
    ExpectedDataType,

    #[snafu(display("Expected to parse a Data Field."))]
    ExpectedDataField,

    #[snafu(display(
        "Expected to parse docs with no spaces of indentation, instead of {actual}."
    ))]
    UnexpectedDocsIndentation { actual: usize },

    #[snafu(display(
        "Expected to parse docs with at least {minimum_expected} space(s) of indentation, not 0."
    ))]
    ExpectedDocsIndentation { minimum_expected: usize },

    #[snafu(display("An unexpected error occurred while parsing the source text."))]
    UnexpectedError,
}

#[cfg(test)]
pub(crate) mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    pub const TEST_CODA_MARKDOWN: &str = r#"
# `MyCoda` Coda

An example Markdown Coda.

## `MyNestedDataType` Data

A data type for nesting inside [`MyDataType`].

+ `floaty_field` f32

    A 32-bit floating-point field.

+ `listy_field` list of text

    A list of data with the same type.

    > This field has some fancy nested docs, too.

    Like, _really_ fancy. <3

## `MyDataType` Data

An example Markdown Data Type.

+ `textual_field` text

    A UTF-8 encoded text field.

+ `integral_field` i32

    A 32-bit signed integer field.

+ `nested_field` [`MyNestedDataType`](#mynesteddatatype-data)

    A nested field.

+ `optional_field` optional u64

    A semantically optional `u64` field.

+ `3d_field` 3d list of i32

    A field containing a 3-dimensional list of numbers.

+ `map_field` map of text to i32

    A field containing a map of text to numbers.

+ `unspecified_field` unspecified

    A field with unspecified typing.
"#;

    #[test]
    fn parses_markdown_into_coda() -> Result<(), ParseError> {
        let coda = parse(TEST_CODA_MARKDOWN)?;

        let mut expected = Coda::new(
            "MyCoda".into(),
            "MyCoda".into(),
            Some("An example Markdown Coda.".into()),
            &[],
        );

        // The "MyNestedDataType" spec.
        let nested_data_type = DataType::new(
            "MyNestedDataType".into(),
            Some("A data type for nesting inside [`MyDataType`].".into()),
            1,
            &[],
            &[],
        )
        .with(DataField {
            name: "floaty_field".into(),
            docs: Some("A 32-bit floating-point field.".into()),
            typing: Type::F32,
            optional: false,
            flattened: false,
        })
        .with(DataField {
            name: "listy_field".into(),
            docs: Some("A list of data with the same type.\n\n    > This field has some fancy nested docs, too.\n\n    Like, _really_ fancy. <3".into()),
            typing: Type::List(Type::Text.into()),
            optional: false,
            flattened: false,
        });
        expected.data.push(nested_data_type.clone());

        // The "MyDataType" spec.
        expected.data.push(
            DataType::new(
                "MyDataType".into(),
                Some("An example Markdown Data Type.".into()),
                2,
                &[],
                &[],
            )
            .with(DataField {
                name: "textual_field".into(),
                docs: Some("A UTF-8 encoded text field.".into()),
                typing: Type::Text,
                optional: false,
                flattened: false,
            })
            .with(DataField {
                name: "integral_field".into(),
                docs: Some("A 32-bit signed integer field.".into()),
                typing: Type::I32,
                optional: false,
                flattened: false,
            })
            .with(DataField {
                name: "nested_field".into(),
                docs: Some("A nested field.".into()),
                typing: Type::Data(nested_data_type),
                optional: false,
                flattened: false,
            })
            .with(DataField {
                name: "optional_field".into(),
                docs: Some("A semantically optional `u64` field.".into()),
                typing: Type::U64,
                optional: true,
                flattened: false,
            })
            .with(DataField {
                name: "3d_field".into(),
                docs: Some("A field containing a 3-dimensional list of numbers.".into()),
                typing: Type::List(Type::List(Type::List(Type::I32.into()).into()).into()),
                optional: false,
                flattened: false,
            })
            .with(DataField {
                name: "map_field".into(),
                docs: Some("A field containing a map of text to numbers.".into()),
                typing: Type::Map((Type::Text, Type::I32).into()),
                optional: false,
                flattened: false,
            })
            .with(DataField {
                name: "unspecified_field".into(),
                docs: Some("A field with unspecified typing.".into()),
                typing: Type::Unspecified,
                optional: false,
                flattened: false,
            }),
        );

        assert_eq!(expected, coda);

        Ok(())
    }

    #[test]
    fn parses_markdown_into_intermediate_representation() -> Result<(), ParseError> {
        let mut parser = Parser::new(TEST_CODA_MARKDOWN);
        let coda = parser.parse()?;

        assert_eq!("MyCoda", coda.global_name);
        assert_eq!("MyCoda", coda.local_name);
        assert_eq!(
            "An example Markdown Coda.",
            TEST_CODA_MARKDOWN[coda.docs].trim()
        );

        // Check first data.
        let data = &coda.data[0];
        assert_eq!("MyNestedDataType", data.name);
        assert_eq!(
            "A data type for nesting inside [`MyDataType`].",
            TEST_CODA_MARKDOWN[data.docs.clone()].trim()
        );
        let field = &data.fields[0];
        assert_eq!("floaty_field", field.name);
        assert_eq!(
            "A 32-bit floating-point field.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::Scalar("f32".into()), field.typing);
        assert!(!field.optional);
        let field = &data.fields[1];
        assert_eq!("listy_field", field.name);
        assert_eq!(
            r#"A list of data with the same type.

    > This field has some fancy nested docs, too.

    Like, _really_ fancy. <3"#,
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::List(1, "text".into()), field.typing);
        assert!(!field.optional);

        // Check second data.
        let data = &coda.data[1];
        assert_eq!("MyDataType", data.name);
        assert_eq!(
            "An example Markdown Data Type.",
            TEST_CODA_MARKDOWN[data.docs.clone()].trim()
        );

        let field = &data.fields[0];
        assert_eq!("textual_field", field.name);
        assert_eq!(
            "A UTF-8 encoded text field.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::Scalar("text".into()), field.typing);
        assert!(!field.optional);

        let field = &data.fields[1];
        assert_eq!("integral_field", field.name);
        assert_eq!(
            "A 32-bit signed integer field.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::Scalar("i32".into()), field.typing);
        assert!(!field.optional);

        let field: &ParsedField = &data.fields[2];
        assert_eq!("nested_field", field.name);
        assert_eq!(
            "A nested field.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(
            ParsedFieldType::Scalar("MyNestedDataType".into()),
            field.typing
        );
        assert!(!field.optional);

        let field: &ParsedField = &data.fields[3];
        assert_eq!("optional_field", field.name);
        assert_eq!(
            "A semantically optional `u64` field.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::Scalar("u64".into()), field.typing);
        assert!(field.optional);

        let field: &ParsedField = &data.fields[4];
        assert_eq!("3d_field", field.name);
        assert_eq!(
            "A field containing a 3-dimensional list of numbers.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::List(3, "i32".into()), field.typing);
        assert!(!field.optional);

        let field: &ParsedField = &data.fields[5];
        assert_eq!("map_field", field.name);
        assert_eq!(
            "A field containing a map of text to numbers.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(
            ParsedFieldType::Map("text".into(), "i32".into()),
            field.typing
        );
        assert!(!field.optional);

        let field: &ParsedField = &data.fields[6];
        assert_eq!("unspecified_field", field.name);
        assert_eq!(
            "A field with unspecified typing.",
            TEST_CODA_MARKDOWN[field.docs.clone()].trim()
        );
        assert_eq!(ParsedFieldType::Scalar("unspecified".into()), field.typing);
        assert!(!field.optional);

        Ok(())
    }

    #[test]
    fn parses_coda_local_names() -> Result<(), ParseError> {
        // Test without Coda suffix.
        let mut parser = Parser::new("# `codas.dev:names/local/Test`");
        let coda = parser.parse()?;
        assert_eq!("codas.dev:names/local/Test", coda.global_name);
        assert_eq!("Test", coda.local_name);

        // Test with Coda suffix.
        let mut parser = Parser::new("# `codas.dev:names/local/Test` Coda");
        let coda = parser.parse()?;
        assert_eq!("codas.dev:names/local/Test", coda.global_name);
        assert_eq!("Test", coda.local_name);

        Ok(())
    }
}
