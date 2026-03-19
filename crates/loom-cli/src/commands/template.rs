pub fn list() -> miette::Result<()> {
    for template in crate::templates::list() {
        let default = if template.id == crate::templates::default_template_id() {
            " (default)"
        } else {
            ""
        };
        println!("{}\t{}{}", template.id, template.description, default);
    }
    Ok(())
}
