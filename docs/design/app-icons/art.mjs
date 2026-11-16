import { writeFileSync } from 'node:fs';

const here = new URL('.', import.meta.url).pathname;
const shade = (d, o = 0.14) => `<path d="${d}" fill="#000" opacity="${o}"/>`;
const light = (d, o = 0.16) => `<path d="${d}" fill="#fff" opacity="${o}"/>`;
const eye = (x, y, r = 26) =>
  `<circle cx="${x}" cy="${y}" r="${r}" fill="#1b1512"/><circle cx="${x + r * 0.3}" cy="${y - r * 0.35}" r="${r * 0.3}" fill="#fff" opacity=".9"/>`;
const svg = (bg, body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><rect width="1024" height="1024" fill="${bg}"/>${body}</svg>\n`;

const KB = { bg: '#13263d', cream: '#efe5cf', chest: '#e3d4b4', brown: '#6a4630', dark: '#4a3122', crown: '#8a6542', blue: '#6fb4e2', beak: '#2c2723', jaw: '#cfbd98' };
const kbBody = `
  <path d="M330 1024 C350 840 430 700 560 640 C700 590 900 620 1024 700 L1024 1024 Z" fill="${KB.chest}"/>
  <path d="M640 640 C790 600 950 650 1024 720 L1024 1024 L600 1024 C650 900 610 760 640 640 Z" fill="${KB.brown}"/>
  ${shade('M640 640 C700 700 720 860 700 1024 L600 1024 C650 900 610 760 640 640 Z', 0.18)}
  <g fill="${KB.blue}"><rect x="720" y="760" width="90" height="26" rx="13" transform="rotate(-18 765 773)"/><rect x="800" y="820" width="110" height="26" rx="13" transform="rotate(-18 855 833)"/><rect x="740" y="860" width="70" height="24" rx="12" transform="rotate(-18 775 872)"/><rect x="860" y="740" width="80" height="24" rx="12" transform="rotate(-18 900 752)"/></g>
  ${shade('M330 1024 C350 900 400 800 470 740 C470 840 460 940 470 1024 Z', 0.08)}`;
const kbHead = (extra = '') => `
  <ellipse cx="570" cy="430" rx="250" ry="222" fill="${KB.cream}"/>
  ${shade('M360 520 C420 640 620 690 780 600 C700 660 520 680 420 600 Z', 0.10)}
  <path d="M350 350 C410 210 640 160 800 290 C770 312 730 300 690 300 C600 300 480 320 350 350 Z" fill="${KB.crown}"/>
  <path d="M790 280 C840 250 870 270 880 300 C850 296 830 300 812 318 Z M800 320 C860 320 880 350 878 380 C850 360 830 360 805 368 Z" fill="${KB.crown}"/>
  <g stroke="${KB.dark}" stroke-width="10" stroke-linecap="round" opacity=".55"><path d="M470 300 C540 260 620 250 700 262"/><path d="M430 330 C520 300 610 292 720 296"/></g>
  <path d="M440 398 C580 378 720 392 812 438 C720 462 580 462 440 446 Z" fill="${KB.dark}"/>
  ${extra}`;
const kbBeakShut = `
  <path d="M410 392 C320 392 190 428 92 494 C210 482 330 470 420 470 Z" fill="${KB.beak}"/>
  <path d="M420 470 C330 474 200 488 92 494 C190 528 330 534 428 512 Z" fill="${KB.jaw}"/>
  ${light('M410 400 C330 404 220 432 140 474 C230 450 330 432 410 426 Z', 0.12)}`;
const kookaburra = {
  laughing: svg(KB.bg, `${kbBody}<g transform="rotate(-12 570 520)">${kbHead(`
    <path d="M430 410 C470 380 510 382 532 410" fill="none" stroke="#1b1512" stroke-width="16" stroke-linecap="round"/>
    <path d="M440 440 L124 396 L150 520 L440 492 Z" fill="#8e3b32"/><path d="M420 470 C340 470 260 480 200 496 C270 500 350 498 430 486 Z" fill="#c9675a"/>
    <path d="M430 372 C330 362 200 360 104 392 C200 420 330 446 440 456 Z" fill="${KB.beak}"/>
    <path d="M440 478 C330 492 230 508 146 522 C230 560 350 560 446 530 Z" fill="${KB.jaw}"/>
    ${light('M420 382 C330 376 230 378 160 394 C240 404 330 414 420 418 Z', 0.12)}`)}</g>
    <g fill="none" stroke="${KB.cream}" stroke-width="16" stroke-linecap="round" opacity=".55"><path d="M80 200 C60 240 60 290 76 330"/><path d="M40 160 C10 220 10 300 36 360"/><path d="M110 700 C90 740 100 790 130 820"/></g>`),
  perched: svg(KB.bg, `${kbBody}
    <path d="M0 880 C220 850 520 860 1024 820 L1024 900 C600 930 260 930 0 950 Z" fill="#7a5638"/>
    ${shade('M0 920 C260 900 600 900 1024 870 L1024 900 C600 930 260 930 0 950 Z', 0.2)}
    <g fill="#7fa16f"><path d="M150 870 C120 800 150 740 200 710 C214 770 200 830 150 870 Z"/><path d="M260 860 C280 790 330 760 380 760 C360 820 320 850 260 860 Z"/></g>
    ${kbHead(eye(468, 418, 28))}${kbBeakShut}`),
  glance: svg(KB.bg, `${kbBody}<g transform="rotate(8 570 520)">${kbHead(`
    <circle cx="470" cy="418" r="40" fill="#fff"/><circle cx="488" cy="420" r="24" fill="#1b1512"/><circle cx="496" cy="410" r="8" fill="#fff"/>
    <path d="M420 362 C450 340 500 336 532 352" fill="none" stroke="${KB.dark}" stroke-width="14" stroke-linecap="round"/>`)}${kbBeakShut}</g>`),
  wings: svg(KB.bg, `
    ${[0, 1].map(m => `<g transform="${m ? 'matrix(-1 0 0 1 1024 0) ' : ''}rotate(6 512 650) translate(-10 -110)">
      <path d="M512 650 C420 570 280 470 70 420 L110 490 L50 510 L120 570 L70 600 L160 650 L130 690 L240 720 C320 750 400 800 450 870 Z" fill="${KB.brown}"/>
      ${shade('M70 420 L110 490 L50 510 L120 570 L70 600 L160 650 L130 690 L240 720 C320 750 400 800 450 870 L512 650 C380 620 240 560 70 420 Z', 0.18)}
      <path d="M512 650 C420 590 330 540 240 520 C320 560 400 610 470 690 Z" fill="${KB.dark}" opacity=".6"/>
      <g fill="${KB.blue}"><rect x="250" y="560" width="120" height="26" rx="13" transform="rotate(24 310 573)"/><rect x="290" y="620" width="120" height="26" rx="13" transform="rotate(28 350 633)"/><rect x="200" y="510" width="90" height="24" rx="12" transform="rotate(20 245 522)"/></g></g>`).join('')}
    <ellipse cx="512" cy="900" rx="210" ry="250" fill="${KB.chest}"/>
    ${shade('M512 650 C620 700 700 820 700 1024 L620 1024 C640 860 600 740 512 650 Z', 0.08)}
    <g transform="translate(60 110) scale(.78)">${kbHead(eye(468, 418, 28))}${kbBeakShut}</g>`),
};

const BUSH = '#3b2318';
const bush = {
  roo: svg(BUSH, `
    <path d="M420 1024 C430 840 500 680 620 600 C760 620 880 780 920 1024 Z" fill="#c27c45"/>
    <path d="M470 1024 C480 870 540 760 630 710 C660 820 650 930 640 1024 Z" fill="#e8c697"/>
    ${shade('M760 640 C860 720 900 860 920 1024 L800 1024 C820 880 800 760 760 640 Z', 0.14)}
    <ellipse cx="600" cy="230" rx="58" ry="160" fill="#b87240" transform="rotate(-12 600 230)"/><ellipse cx="604" cy="250" rx="30" ry="112" fill="#d99c84" transform="rotate(-12 604 250)"/>
    <ellipse cx="716" cy="250" rx="58" ry="160" fill="#c27c45" transform="rotate(14 716 250)"/><ellipse cx="712" cy="268" rx="30" ry="112" fill="#d99c84" transform="rotate(14 712 268)"/>
    <ellipse cx="610" cy="460" rx="200" ry="160" fill="#c27c45" transform="rotate(-14 610 460)"/>
    <ellipse cx="400" cy="520" rx="170" ry="100" fill="#c27c45" transform="rotate(-8 400 520)"/>
    <ellipse cx="360" cy="560" rx="130" ry="60" fill="#e8c697" transform="rotate(-8 360 560)"/>
    ${shade('M300 590 C380 640 520 640 640 580 C560 650 400 670 300 590 Z', 0.16)}
    <ellipse cx="248" cy="500" rx="42" ry="32" fill="#2b1a12"/>${eye(520, 420, 24)}
    ${light('M480 330 C560 300 660 310 740 360 C660 340 560 340 480 360 Z', 0.14)}`),
  wombat: svg(BUSH, `
    <path d="M200 1024 C230 820 380 700 560 690 C760 690 940 800 1000 1024 Z" fill="#7d6a5a"/>
    ${shade('M700 700 C860 760 960 880 1000 1024 L820 1024 C820 880 780 780 700 700 Z', 0.16)}
    <path d="M330 300 C340 230 400 210 440 250 C420 290 390 310 330 300 Z" fill="#6c5a4b"/><path d="M700 280 C720 210 790 210 810 260 C780 290 740 300 700 280 Z" fill="#6c5a4b"/>
    <ellipse cx="560" cy="500" rx="300" ry="250" fill="#8a7666"/>
    ${light('M330 380 C420 290 640 270 780 340 C640 320 460 330 330 380 Z', 0.12)}
    ${shade('M290 560 C360 720 640 770 830 640 C760 740 480 780 290 560 Z', 0.14)}
    <path d="M250 520 C250 440 330 420 400 430 C460 440 470 520 450 580 C420 640 280 640 250 520 Z" fill="#2b2420"/>
    ${light('M290 470 C320 450 370 448 400 460 C360 470 320 480 290 470 Z', 0.2)}
    ${eye(500, 420, 20)}${eye(650, 420, 20)}`),
  koala: svg(BUSH, `
    <path d="M220 1024 C250 830 380 720 540 710 C720 710 880 820 920 1024 Z" fill="#9aa1a5"/>
    <path d="M400 1024 C420 900 480 820 560 800 C630 820 680 920 690 1024 Z" fill="#e7e4dc"/>
    <path d="M820 1024 C860 940 940 900 1024 900 L1024 1024 Z" fill="#6f4d34"/>
    <g fill="#7fa16f"><path d="M900 900 C880 830 920 780 980 770 C990 830 960 880 900 900 Z"/></g>
    <circle cx="300" cy="340" r="130" fill="#9aa1a5"/><circle cx="300" cy="350" r="80" fill="#e7e4dc"/>
    <circle cx="790" cy="340" r="130" fill="#9aa1a5"/><circle cx="790" cy="350" r="80" fill="#e7e4dc"/>
    <ellipse cx="545" cy="500" rx="270" ry="235" fill="#a8afb3"/>
    ${light('M330 400 C420 300 680 290 780 380 C660 340 450 340 330 400 Z', 0.14)}
    ${shade('M300 560 C380 720 700 740 800 580 C730 700 400 720 300 560 Z', 0.14)}
    <path d="M500 450 C470 520 480 620 545 640 C610 620 620 520 590 450 C570 420 520 420 500 450 Z" fill="#2b2826"/>
    ${light('M520 470 C540 460 560 462 572 474 C556 480 536 482 520 470 Z', 0.25)}
    ${eye(420, 470, 18)}${eye(670, 470, 18)}`),
  emu: svg(BUSH, `
    <path d="M280 1024 C300 860 420 760 560 740 C740 720 910 820 980 1024 Z" fill="#5a4636"/>
    <g fill="#473629"><path d="M400 830 L350 920 L460 870 Z"/><path d="M540 790 L510 900 L610 830 Z"/><path d="M690 790 L700 910 L770 830 Z"/><path d="M820 830 L880 940 L900 850 Z"/><path d="M600 900 L570 1000 L660 950 Z"/></g>
    ${shade('M700 760 C860 800 960 900 980 1024 L840 1024 C840 900 800 820 700 760 Z', 0.16)}
    <path d="M500 790 C460 650 440 520 440 400 L610 380 C600 500 620 640 680 770 Z" fill="#5a4636"/>
    <path d="M470 760 C450 640 450 520 460 420 L560 410 C560 520 570 640 600 760 Z" fill="#86a3bd"/>
    <g fill="#473629"><path d="M600 440 L650 470 L606 500 Z"/><path d="M608 540 L662 572 L614 600 Z"/><path d="M620 640 L676 676 L630 700 Z"/></g>
    ${shade('M560 410 C560 520 570 640 600 760 L540 770 C520 640 512 520 516 414 Z', 0.12)}
    <ellipse cx="500" cy="320" rx="160" ry="135" fill="#4d3b2e"/>
    <ellipse cx="450" cy="320" rx="90" ry="70" fill="#86a3bd"/>
    ${light('M420 220 C470 190 560 196 620 236 C540 222 470 222 420 246 Z', 0.14)}
    <g fill="#3e2f25"><path d="M470 200 L470 120 L530 196 Z"/><path d="M540 196 L600 130 L596 216 Z"/><path d="M600 220 L680 180 L650 250 Z"/><path d="M630 290 L710 280 L660 330 Z"/></g>
    <path d="M370 300 C330 292 280 296 210 330 C280 346 330 352 372 352 Z" fill="#2c2723"/>
    <path d="M372 352 C330 352 270 346 210 330 C270 372 320 384 376 376 Z" fill="#6b5a4a"/>
    ${eye(440, 310, 26)}`),
};

const GARDEN = '#1c3324';
const pot = `<path d="M240 900 L784 900 L760 1024 L264 1024 Z" fill="#c4683e"/><rect x="210" y="860" width="604" height="70" rx="14" fill="#d8784a"/>${shade('M520 860 L814 860 L814 930 L770 930 L760 1024 L520 1024 Z', 0.14)}`;
const leaf = (x, y, s, r, c = '#3f8a4c') =>
  `<g transform="translate(${x} ${y}) rotate(${r}) scale(${s})"><path d="M0 0 C-150 -40 -200 -230 -60 -360 C0 -400 60 -400 120 -360 C260 -230 200 -40 0 0 Z" fill="${c}"/>
   <path d="M30 -300 L0 0" stroke="#2e6a3a" stroke-width="10"/><g fill="${GARDEN}"><ellipse cx="-80" cy="-170" rx="44" ry="18" transform="rotate(30 -80 -170)"/><ellipse cx="120" cy="-170" rx="44" ry="18" transform="rotate(-30 120 -170)"/><ellipse cx="-60" cy="-260" rx="36" ry="14" transform="rotate(40 -60 -260)"/><ellipse cx="110" cy="-260" rx="36" ry="14" transform="rotate(-40 110 -260)"/></g>
   ${shade('M30 -360 C240 -230 200 -40 0 0 Z', 0.14)}</g>`;
const garden = {
  monstera: svg(GARDEN, `${leaf(560, 860, 1.45, -4, '#4f9a5a')}${leaf(330, 880, 1.05, -38)}${leaf(760, 880, 1.0, 34, '#468f52')}${pot}`),
  sunflower: svg(GARDEN, `
    <path d="M520 1024 C520 900 500 760 480 600" stroke="#3f7a3a" stroke-width="40" fill="none"/>
    <path d="M500 840 C380 800 260 830 200 900 C320 930 430 900 500 840 Z" fill="#4f9a5a"/><path d="M510 760 C640 700 780 720 840 800 C720 840 600 820 510 760 Z" fill="#468f52"/>
    <g transform="translate(470 400)">${Array.from({ length: 16 }, (_, i) => `<ellipse cx="0" cy="-230" rx="62" ry="140" fill="${i % 2 ? '#f0b432' : '#f6c94a'}" transform="rotate(${i * 22.5})"/>`).join('')}
    <circle r="170" fill="#6a3e1e"/>${shade('M0 -170 A170 170 0 0 1 0 170 A120 170 0 0 0 0 -170 Z', 0.2)}
    <g fill="#8a5428">${Array.from({ length: 14 }, (_, i) => `<circle cx="${Math.cos(i * 0.9) * (40 + i * 7)}" cy="${Math.sin(i * 0.9) * (40 + i * 7)}" r="14"/>`).join('')}</g></g>`),
  cactus: svg(GARDEN, `
    <path d="M400 900 L400 330 C400 230 620 230 620 330 L620 900 Z" fill="#5aa268"/>
    <path d="M400 640 C320 640 280 600 280 540 L280 430 C280 380 350 380 350 430 L350 560 C350 580 370 590 400 590 Z" fill="#5aa268"/>
    <path d="M620 560 C700 560 740 520 740 460 L740 360 C740 310 670 310 670 360 L670 480 C670 500 650 510 620 510 Z" fill="#5aa268"/>
    ${shade('M520 250 C600 260 620 300 620 330 L620 900 L540 900 Z', 0.16)}
    <g stroke="#3d7a4a" stroke-width="10"><path d="M460 300 V880"/><path d="M560 300 V880"/></g>
    <g fill="#f08aa8"><circle cx="510" cy="240" r="40"/><circle cx="470" cy="214" r="34"/><circle cx="550" cy="214" r="34"/></g><circle cx="510" cy="222" r="20" fill="#f6d06a"/>
    ${pot}`),
  tulips: svg(GARDEN, `
    <g stroke="#3f7a3a" stroke-width="30" fill="none"><path d="M510 900 C510 700 500 560 500 420"/><path d="M380 900 C360 720 320 620 290 520"/><path d="M640 900 C660 720 700 640 740 540"/></g>
    <path d="M470 860 C380 760 360 620 400 520 C470 620 490 740 470 860 Z" fill="#4f9a5a"/><path d="M560 860 C660 760 680 640 650 540 C580 640 550 740 560 860 Z" fill="#468f52"/>
    ${[[500, 380, 1.15, '#e25563'], [290, 480, 0.9, '#f08aa8'], [740, 500, 0.9, '#f6c94a']].map(([x, y, s, c]) => `<g transform="translate(${x} ${y}) scale(${s})"><path d="M-110 -40 C-110 80 -60 130 0 130 C60 130 110 80 110 -40 L60 0 L30 -110 L0 -10 L-30 -110 L-60 0 Z" fill="${c}"/>${shade('M0 -10 L30 -110 L60 0 L110 -40 C110 80 60 130 0 130 Z', 0.16)}</g>`).join('')}
    ${pot}`),
};

const KITCHEN = '#2a1f1a';
const cook = ({ skin, hat, hair = '', top, extra = '' }) => svg(KITCHEN, `
  <path d="M240 1024 C260 830 390 720 540 710 C720 710 880 820 920 1024 Z" fill="${top}"/>
  ${shade('M700 720 C840 780 900 900 920 1024 L780 1024 C780 900 760 800 700 720 Z', 0.12)}
  <rect x="470" y="600" width="140" height="140" fill="${skin}"/>${shade('M470 680 L610 680 L610 740 L470 740 Z', 0.16)}
  ${hair}
  <ellipse cx="760" cy="480" rx="44" ry="60" fill="${skin}"/>
  <ellipse cx="560" cy="470" rx="210" ry="225" fill="${skin}"/>
  ${shade('M560 245 C700 260 790 380 770 520 C750 640 660 700 560 695 C680 640 720 520 690 400 C670 320 620 270 560 245 Z', 0.12)}
  <circle cx="430" cy="530" r="34" fill="#e88a7a" opacity=".55"/>
  ${eye(450, 460, 20)}${eye(580, 460, 20)}
  <path d="M370 500 C340 540 350 570 380 570" fill="none" stroke="#000" stroke-opacity=".25" stroke-width="12" stroke-linecap="round"/>
  ${extra}${hat}`);
const kitchen = {
  'head-chef': cook({
    skin: '#e9b48e', top: '#f1ece2',
    hat: `<path d="M360 330 C300 260 320 140 420 130 C440 60 560 40 610 110 C680 60 790 110 770 210 C830 260 800 340 740 330 Z" fill="#f4f1ea"/><rect x="360" y="300" width="390" height="90" rx="20" fill="#e6e1d6"/>${shade('M610 110 C680 60 790 110 770 210 C830 260 800 340 740 330 L740 390 L620 390 Z', 0.08)}`,
    extra: `<path d="M420 600 C470 570 520 580 560 600 C600 580 650 570 690 600 C640 620 600 630 560 616 C520 630 470 620 420 600 Z" fill="#5a3a24"/>
      <path d="M440 740 L560 830 L680 740 L640 720 L560 770 L480 720 Z" fill="#4f8fd0"/><g fill="#c9c1b0"><circle cx="500" cy="900" r="16"/><circle cx="620" cy="900" r="16"/><circle cx="500" cy="980" r="16"/><circle cx="620" cy="980" r="16"/></g>`,
  }),
  pastry: cook({
    skin: '#8d5a3c', top: '#f2c6cf',
    hair: `<circle cx="760" cy="300" r="90" fill="#2a1c16"/>`,
    hat: `<path d="M340 330 C350 200 470 150 580 160 C700 170 790 240 790 340 C640 300 480 300 340 330 Z" fill="#f08aa8"/>${shade('M580 160 C700 170 790 240 790 340 C740 330 700 320 660 316 C670 260 640 200 580 160 Z', 0.12)}`,
    extra: `<path d="M440 620 C500 650 560 650 610 620" fill="none" stroke="#3a1f18" stroke-width="12" stroke-linecap="round"/>
      <g transform="rotate(-30 300 860)"><rect x="285" y="820" width="30" height="200" rx="15" fill="#c9c1b0"/><path d="M300 830 C230 760 230 640 300 610 C370 640 370 760 300 830 Z M300 830 C260 760 260 660 300 630 M300 830 C340 760 340 660 300 630" fill="none" stroke="#dcd6ca" stroke-width="10"/></g>`,
  }),
  'line-cook': cook({
    skin: '#c98e66', top: '#304a6e',
    hat: `<path d="M340 360 C350 220 470 180 570 180 C690 180 790 240 790 360 Z" fill="#c8453a"/><g fill="#f4f1ea"><circle cx="440" cy="290" r="12"/><circle cx="560" cy="250" r="12"/><circle cx="680" cy="300" r="12"/><circle cx="520" cy="330" r="12"/></g><path d="M790 330 L880 300 L870 380 Z" fill="#c8453a"/>${shade('M570 180 C690 180 790 240 790 360 L680 360 C690 280 650 210 570 180 Z', 0.12)}`,
    extra: `<path d="M380 560 C400 680 500 720 560 720 C640 720 730 660 740 560 C700 600 640 610 560 600 C480 610 420 600 380 560 Z" fill="#4a2e20"/>
      <path d="M470 760 L520 1024 L600 1024 L650 760 Z" fill="#f1ece2"/>`,
  }),
  sous: cook({
    skin: '#f0c9a8', top: '#3c3a38',
    hat: `<path d="M350 340 L380 210 L760 210 L780 340 Z" fill="#f4f1ea"/>${shade('M600 210 L760 210 L780 340 L640 340 Z', 0.08)}<rect x="350" y="310" width="430" height="44" rx="10" fill="#4f8fd0"/>`,
    hair: `<path d="M720 360 C800 380 820 480 790 560 C760 520 740 440 720 360 Z" fill="#b5652e"/>`,
    extra: `<g fill="none" stroke="#2a2420" stroke-width="10"><circle cx="450" cy="460" r="46"/><circle cx="580" cy="460" r="46"/><path d="M496 460 L534 460 M626 450 L740 430"/></g>
      <path d="M470 610 C520 630 570 630 610 610" fill="none" stroke="#7a3a2a" stroke-width="12" stroke-linecap="round"/>
      <g transform="rotate(20 820 880)"><rect x="805" y="640" width="30" height="300" rx="15" fill="#b9b2a4"/><path d="M740 900 C740 990 900 990 900 900 Z" fill="#b9b2a4"/></g>`,
  }),
};

const TERM = '#0d1117';
const glyph = (color, body) => svg(TERM, `
  <rect x="112" y="152" width="800" height="720" rx="72" fill="#161b22"/><rect x="112" y="152" width="800" height="96" rx="48" fill="#21262d"/><rect x="112" y="200" width="800" height="48" fill="#21262d"/>
  <g fill="#30363d"><circle cx="186" cy="200" r="18"/><circle cx="242" cy="200" r="18"/><circle cx="298" cy="200" r="18"/></g>
  <g transform="translate(14 14)" opacity=".35">${body.replace(/COLOR/g, '#000')}</g>${body.replace(/COLOR/g, color)}`);
const terminal = {
  prompt: glyph('#3fb950', `<path d="M260 380 L470 560 L260 740" fill="none" stroke="COLOR" stroke-width="84" stroke-linecap="round" stroke-linejoin="round"/><rect x="530" y="690" width="240" height="84" rx="16" fill="COLOR"/>`),
  dollar: glyph('#39c5cf', `<path d="M640 430 C620 380 560 360 500 360 C420 360 360 400 360 460 C360 600 660 520 660 650 C660 710 600 750 510 750 C440 750 380 720 360 670" fill="none" stroke="COLOR" stroke-width="72" stroke-linecap="round"/><path d="M505 300 V810" stroke="COLOR" stroke-width="56" stroke-linecap="round"/><rect x="700" y="700" width="110" height="84" rx="12" fill="COLOR"/>`),
  chevron: glyph('#bc8cff', `<path d="M300 360 L560 560 L300 760 L300 640 L400 560 L300 480 Z" fill="COLOR" stroke="COLOR" stroke-width="40" stroke-linejoin="round"/><rect x="620" y="690" width="160" height="84" rx="12" fill="COLOR"/>`),
  lambda: glyph('#58a6ff', `<path d="M300 360 C380 340 430 380 460 450 L640 780" fill="none" stroke="COLOR" stroke-width="76" stroke-linecap="round"/><path d="M470 560 L320 780" fill="none" stroke="COLOR" stroke-width="76" stroke-linecap="round"/><rect x="680" y="700" width="120" height="84" rx="12" fill="COLOR"/>`),
};

const SETS = { modern: kookaburra, bush, garden, kitchen, terminal };
for (const [set, icons] of Object.entries(SETS))
  for (const [name, text] of Object.entries(icons)) writeFileSync(`${here}icons/${set}-${name}.svg`, text);
console.log(Object.values(SETS).reduce((n, s) => n + Object.keys(s).length, 0), 'icons written');
