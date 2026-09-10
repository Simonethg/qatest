/// Commands qatest prints. qatest does not wrap Playwright or pytest (pilar 04).
pub fn recipe(url: Option<&str>) -> String {
    let url = url.unwrap_or("$SUT_URL");
    format!(
        "\
qatest tests recipe
Pilar 04: run these in the Tests panes. qatest is the house, not a new framework.

# Deterministic suite
npx playwright test
pytest -q

# EvalHarness (score + threshold) — lesson 4.3
qatest evals run

# axe-core on the SUT, scoped, both required viewports
# 390 mobile / 1440 desktop. Critical and serious block Review.
export QATEST_AXE_INCLUDE='[data-testid=changed-component]'   # never full page
qatest axe --url {url}

# equivalent raw CLI if you want the pane to own it:
npx --yes @axe-core/cli --tags wcag22aa --viewport-size=390,844 {url}
npx --yes @axe-core/cli --tags wcag22aa --viewport-size=1440,900 {url}

# ISO 25010 labels live on A# rows in Spec, not as a separate product.
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_names_both_viewports_and_pytest() {
        let r = recipe(Some("http://localhost:3000"));
        assert!(r.contains("390"));
        assert!(r.contains("1440"));
        assert!(r.contains("pytest"));
        assert!(r.contains("playwright"));
    }
}
