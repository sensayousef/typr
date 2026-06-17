export function initNav(): void {
  const navItems = document.querySelectorAll<HTMLElement>(".nav-item");
  const sections = document.querySelectorAll<HTMLElement>(".content-section");

  navItems.forEach((item) => {
    item.addEventListener("click", () => {
      const target = item.getAttribute("data-section");
      navItems.forEach((n) => n.classList.remove("active"));
      sections.forEach((s) => s.classList.remove("active"));
      item.classList.add("active");
      document.getElementById(`section-${target}`)?.classList.add("active");
    });
  });
}
