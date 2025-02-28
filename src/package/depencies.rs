#[derive(Debug, PartialEq)]
pub struct PackageQuery {
    pub name: String,
    pub version: String,
    pub comparison_operator: String,
}

impl PackageQuery {
    // Метод для парсинга строки типа "example>=1.1.1alpha"
    pub fn parse(query: &str) -> Result<Self, &'static str> {
        // Сначала ищем двухсимвольные операторы
        let operators = ["<=", ">=", "=", "<", ">"];

        // Ищем оператор в строке
        let operator = operators
            .iter()
            .find(|&&op| query.contains(op))
            .ok_or("Не удалось найти оператор сравнения")?;

        // Разделяем строку на имя пакета и версию
        let parts: Vec<&str> = query.split(operator).collect();
        if parts.len() != 2 {
            return Err("Некорректный формат строки");
        }

        let name = parts[0].trim().to_string();
        let version = parts[1].trim().to_string();

        // Проверяем, что имя и версия не пустые
        if name.is_empty() || version.is_empty() {
            return Err("Имя пакета или версия не могут быть пустыми");
        }

        Ok(PackageQuery {
            name,
            version,
            comparison_operator: operator.to_string(),
        })
    }
}