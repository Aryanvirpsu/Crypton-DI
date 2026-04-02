export const BtnF = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink)", background: "var(--accent)",
    padding: "12px 24px", border: "1px solid var(--accent)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "var(--accent)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "var(--accent)"; e.currentTarget.style.color = "var(--ink)"; }}
  >{children}</button>
);

export const BtnO = ({ children, onClick, style = {} }) => (
  <button onClick={onClick} style={{
    display: "inline-flex", alignItems: "center", gap: 10, fontFamily: "var(--mono)", fontSize: 10,
    letterSpacing: ".1em", textTransform: "uppercase", color: "var(--paper)", background: "none",
    padding: "12px 24px", border: "1px solid var(--line2)", cursor: "pointer", transition: "all .25s", ...style
  }}
    onMouseEnter={e => { e.currentTarget.style.background = "var(--paper)"; e.currentTarget.style.color = "var(--ink)"; }}
    onMouseLeave={e => { e.currentTarget.style.background = "none"; e.currentTarget.style.color = "var(--paper)"; }}
  >{children}</button>
);
