// Splits text into individual characters, each dropping in from above with a stagger.
// charStyle(char, globalIndex) → optional extra style object per character.
export default function AnimatedHeading({ text, style, charStyle }) {
  let nonSpaceIdx = 0;
  return (
    <span style={style}>
      {text.split('').map((char, i) => {
        if (char === ' ') {
          return <span key={i} style={{ display: 'inline-block', width: '0.28em' }}>&nbsp;</span>;
        }
        const delay = nonSpaceIdx++ * 38;
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              animation: `charDrop 0.55s cubic-bezier(0.22,1,0.36,1) ${delay}ms both`,
              ...(charStyle ? charStyle(char, i) : {}),
            }}
          >
            {char}
          </span>
        );
      })}
    </span>
  );
}
