import { useState, useEffect, useRef } from 'react';

const CHARSET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$@';

export default function AnimatedHeading({ text, style, charStyle }) {
  const chars = text.split('');
  const nonSpaceIdxs = chars.map((c, i) => c !== ' ' ? i : null).filter(i => i !== null);

  const lockOrder = useRef([...nonSpaceIdxs].sort(() => Math.random() - 0.5));
  const lockedRef = useRef(new Set());
  const [displayed, setDisplayed] = useState(() =>
    chars.map(c => c === ' ' ? ' ' : CHARSET[Math.floor(Math.random() * CHARSET.length)])
  );
  const [, forceUpdate] = useState(0);

  useEffect(() => {
    lockedRef.current = new Set();
    lockOrder.current = [...nonSpaceIdxs].sort(() => Math.random() - 0.5);

    const scramble = setInterval(() => {
      setDisplayed(prev => prev.map((_, i) =>
        lockedRef.current.has(i)
          ? chars[i]
          : (chars[i] === ' ' ? ' ' : CHARSET[Math.floor(Math.random() * CHARSET.length)])
      ));
    }, 45);

    let lockIdx = 0;
    const lock = setInterval(() => {
      if (lockIdx >= lockOrder.current.length) {
        clearInterval(scramble);
        clearInterval(lock);
        setDisplayed(chars);
        return;
      }
      lockedRef.current.add(lockOrder.current[lockIdx++]);
      forceUpdate(n => n + 1);
    }, 70);

    return () => { clearInterval(scramble); clearInterval(lock); };
  }, [text]); // eslint-disable-line

  return (
    <span style={style}>
      {displayed.map((c, i) => (
        <span
          key={i}
          style={{
            display: 'inline-block',
            color: lockedRef.current.has(i) ? undefined : 'rgba(200,245,90,0.6)',
            transition: 'color 0.1s',
            ...(charStyle ? charStyle(chars[i], i) : {}),
          }}
        >
          {c === ' ' ? '\u00A0' : c}
        </span>
      ))}
    </span>
  );
}
