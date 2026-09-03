use crate::{error::Result, rules::Tier};
use scraper::{ElementRef, Html, Selector};

#[derive(Debug, Clone, Default)]
pub struct FeeRule {
    pub subscribe: Vec<Tier>,
    pub redeem: Vec<Tier>,
}

pub fn parse_fees(html: &str) -> Result<FeeRule> {
    let document = Html::parse_document(html);
    let box_sel = Selector::parse("div.boxitem").expect("valid selector");
    let label_sel = Selector::parse("h4 label.left").expect("valid selector");
    let table_sel = Selector::parse("table").expect("valid selector");
    let tr_sel = Selector::parse("tr").expect("valid selector");
    let td_sel = Selector::parse("td").expect("valid selector");

    let mut rule = FeeRule::default();
    for item in document.select(&box_sel) {
        let label = item
            .select(&label_sel)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();
        let Some(table) = item.select(&table_sel).next() else {
            continue;
        };
        let tiers = parse_table(table, &td_sel, &tr_sel);
        if label.contains("申购费率") {
            rule.subscribe = tiers;
        } else if label.contains("赎回费率") {
            rule.redeem = tiers;
        }
    }
    Ok(rule)
}

fn parse_table(table: ElementRef<'_>, td_sel: &Selector, tr_sel: &Selector) -> Vec<Tier> {
    table
        .select(tr_sel)
        .filter_map(|tr| {
            let cells: Vec<String> = tr
                .select(td_sel)
                .map(|td| td.text().collect::<String>())
                .collect();
            if cells.len() < 2 {
                return None;
            }
            parse_row(&cells[0], &cells[1])
        })
        .collect()
}

fn parse_row(condition: &str, rate: &str) -> Option<Tier> {
    let lower_bound = parse_lower_bound(condition)?;
    if rate.contains('%') {
        let pct = first_number(rate)?;
        Some(Tier::pct(lower_bound, pct))
    } else {
        let amount = first_number(rate)?;
        Some(Tier::fixed(lower_bound, amount))
    }
}

fn parse_lower_bound(condition: &str) -> Option<f64> {
    let condition = condition.trim();
    if condition.starts_with("小于") {
        // "小于..." / "小于等于..." tiers start at zero.
        return Some(0.0);
    }
    let (number, rest) = first_number_with_rest(condition)?;
    match rest.chars().next() {
        Some('万') => Some(number * 1e4),
        Some('亿') => Some(number * 1e8),
        _ => Some(number),
    }
}

fn first_number(text: &str) -> Option<f64> {
    let digits = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit() && *c != '.')
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn first_number_with_rest(text: &str) -> Option<(f64, &str)> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let rest = &text[start..];
    let len = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    let number: f64 = rest[..len].parse().ok()?;
    Some((number, &rest[len..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::FeeKind;

    const SAMPLE: &str = r#"<div class="box"><div class="boxitem w790">
      <h4 class="t"><label class="left">申购费率</label></h4>
      <table class="w650 comm jjfl"><thead><tr><th>适用金额</th><th>原费率|优惠</th></tr></thead>
      <tbody>
        <tr><td>小于100万元</td><td><strike>1.50%</strike>|0.15%</td></tr>
        <tr><td>大于等于100万元，小于500万元</td><td>1.20%</td></tr>
        <tr><td>大于等于1000万元</td><td>每笔1000元</td></tr>
      </tbody></table>
    </div></div>
    <div class="box"><div class="boxitem w790">
      <h4 class="t"><label class="left">赎回费率</label></h4>
      <table class="w650 comm jjfl"><thead><tr><th>适用期限</th><th>赎回费率</th></tr></thead>
      <tbody>
        <tr><td>小于等于6天</td><td>1.50%</td></tr>
        <tr><td>大于等于7天，小于等于364天</td><td>0.50%</td></tr>
        <tr><td>大于等于365天，小于等于729天</td><td>0.25%</td></tr>
        <tr><td>大于等于730天</td><td>0.00%</td></tr>
      </tbody></table>
    </div></div>"#;

    #[test]
    fn test_parse_fees() {
        let rule = parse_fees(SAMPLE).unwrap();
        assert_eq!(rule.subscribe.len(), 3);
        assert_eq!(rule.subscribe[0].lower_bound, 0.0);
        assert_eq!(rule.subscribe[0].kind, FeeKind::Pct);
        assert_eq!(rule.subscribe[0].rate, 1.5);
        assert_eq!(rule.subscribe[1].lower_bound, 1_000_000.0);
        assert_eq!(rule.subscribe[1].rate, 1.2);
        assert_eq!(rule.subscribe[2].lower_bound, 10_000_000.0);
        assert_eq!(rule.subscribe[2].kind, FeeKind::Fixed);
        assert_eq!(rule.subscribe[2].rate, 1000.0);

        assert_eq!(rule.redeem.len(), 4);
        assert_eq!(rule.redeem[0].lower_bound, 0.0);
        assert_eq!(rule.redeem[0].rate, 1.5);
        assert_eq!(rule.redeem[1].lower_bound, 7.0);
        assert_eq!(rule.redeem[1].rate, 0.5);
        assert_eq!(rule.redeem[2].lower_bound, 365.0);
        assert_eq!(rule.redeem[3].lower_bound, 730.0);
    }
}
