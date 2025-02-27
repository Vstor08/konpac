use std::str::FromStr;
use version_compare::Version; // Используем только Version
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    GreaterThan,    // >
    LessThan,       // <
    GreaterOrEqual, // >=
    LessOrEqual,    // <=
    Equal,          // =
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid operator: {0}")]
    InvalidOperator(String),
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    #[error("Invalid dependency format: {0}")]
    InvalidFormat(String),
}

impl ComparisonOperator {
    pub fn from_str(s: &str) -> Result<(Self, usize), ParseError> {
        // Список поддерживаемых операторов и их длины
        let ops = [
            (">=", Self::GreaterOrEqual, 2),
            ("<=", Self::LessOrEqual, 2),
            (">", Self::GreaterThan, 1),
            ("<", Self::LessThan, 1),
            ("=", Self::Equal, 1),
        ];

        // Ищем оператор в строке
        for (op_str, op, len) in ops {
            if s.starts_with(op_str) {
                return Ok((op.clone(), len));
            }
        }

        // Если оператор не найден, возвращаем ошибку
        Err(ParseError::InvalidOperator(s.to_string()))
    }
}

impl std::fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::GreaterThan => ">",
                Self::LessThan => "<",
                Self::GreaterOrEqual => ">=",
                Self::LessOrEqual => "<=",
                Self::Equal => "=",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dependency<'a> {
    pub name: String,
    pub operator: ComparisonOperator,
    pub version: Version<'a>, // Указываем время жизни
}

impl<'a> Dependency<'a> {
    pub fn from_str(s: &'a str) -> Result<Self, ParseError> {
        // Ищем начало оператора
        let operator_start = s.find(|c| "<>=".contains(c))
            .ok_or(ParseError::InvalidFormat(s.to_string()))?;

        // Разделяем строку на имя и остаток
        let (name, rest) = s.split_at(operator_start);
        let name = name.trim().to_string();

        // Парсим оператор
        let (operator, op_len) = ComparisonOperator::from_str(rest)?;

        // Извлекаем версионную часть
        let version_str = &rest[op_len..].trim();

        // Парсим версию
        let version = Version::from(version_str)
            .ok_or(ParseError::InvalidVersion(version_str.to_string()))?;

        // Возвращаем структуру Dependency
        Ok(Dependency {
            name,
            operator,
            version,
        })
    }
}

