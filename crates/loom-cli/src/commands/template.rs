pub fn list() -> miette::Result<()> {
    let templates = crate::templates::list();
    let id_width = templates
        .iter()
        .map(|template| template.id.len())
        .max()
        .unwrap_or(0);

    for template in templates {
        let default = if template.id == crate::templates::default_template_id() {
            " (default)"
        } else {
            ""
        };
        println!(
            "{:<width$}  {}{}",
            template.id,
            template.description,
            default,
            width = id_width
        );
    }
    Ok(())
}
