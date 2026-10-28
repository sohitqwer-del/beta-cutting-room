// The demo video script as data. One source for narration (TTS), captions (en, hi, ta), the recorder and the editor.
// The 0.8 launch is a separate edit: launch8.mjs consumes the following script.
export { social as launchSocial, walkthrough as launchWalkthrough } from './launch-scenes.mjs';
// Each scene is recorded in one "take": a title card page, the desktop app, or the phone app.
// Each beat is one narration line and one caption (2 lines at most). The recorder runs the beat's action
// when the line starts, then waits until the line (plus `gap`) is over, or longer when `min` asks for it.
//   say  - what the voice says (spelled out for the speech engine)
//   en   - English caption (defaults to `say`)
//   hi   - Hindi caption draft (NEEDS NATIVE REVIEW)
//   ta   - Tamil caption draft (NEEDS NATIVE REVIEW)
//   act  - recorder action name (record.mjs), or a card step number for card takes
//   min  - shortest beat length in seconds (for slow app actions such as nesting)
//   teaser - true when this beat is also used in the 60-second vertical teaser

export const SITE_URL = 'https://sohitqwer-del.github.io/beta-cutting-room/';
export const REPO_URL = 'https://github.com/sohitqwer-del/beta-cutting-room';
export const SITE_SHORT = 'sohitqwer-del.github.io/beta-cutting-room';
export const REPO_SHORT = 'github.com/sohitqwer-del/beta-cutting-room';

/** Gap after each narration line, in seconds. */
export const GAP = 0.35;

export const scenes = [
  {
    id: 'hook', take: 'card', card: 'hook', clip: null, vertical: true,
    beats: [
      { say: 'Save fabric on every marker.', hi: 'हर मार्कर पर कपड़ा बचाइए।', ta: 'ஒவ்வொரு மார்க்கரிலும் துணியைச் சேமியுங்கள்.', act: 1, teaser: true },
      { say: 'BETA Cutting Room is free and open source. It runs in your own factory.', hi: 'BETA Cutting Room मुफ़्त और ओपन-सोर्स है। यह आपकी अपनी फ़ैक्टरी में चलता है।', ta: 'BETA Cutting Room இலவசம், ஓப்பன் சோர்ஸ். இது உங்கள் தொழிற்சாலையிலேயே இயங்கும்.', act: 2, teaser: true },
    ],
 },
  {
    id: 'basics', take: 'card', card: 'basics', section: 'The basics', clip: 'basics',
    beats: [
      { say: 'A garment is cut from flat shapes, called pattern pieces.', hi: 'कपड़ा सपाट आकारों से काटा जाता है, जिन्हें पैटर्न पीस कहते हैं।', ta: 'ஆடை தட்டையான வடிவங்களில் இருந்து வெட்டப்படுகிறது; அவை பேட்டர்ன் துண்டுகள்.', act: 1 },
      { say: 'A marker is the cutting plan: all the pieces, laid on the fabric width.', hi: 'मार्कर काटने का प्लान है: सारे पीस, कपड़े की चौड़ाई पर बिछे हुए।', ta: 'மார்க்கர் என்பது வெட்டும் திட்டம்: எல்லா துண்டுகளும் துணியின் அகலத்தில்.', act: 2 },
      { say: 'Pieces closer together make a shorter marker, and use less fabric.', hi: 'पीस पास-पास हों तो मार्कर छोटा होता है और कपड़ा कम लगता है।', ta: 'துண்டுகள் நெருக்கமாக இருந்தால் மார்க்கர் குறையும், துணியும் குறையும்.', act: 3, teaser: true },
      { say: 'For example: 10 centimetres shorter, on 50 layers, saves 5 metres.', en: 'Example: 10 cm shorter, on 50 layers, saves 5 metres of fabric.', hi: 'उदाहरण: 10 सेमी छोटा, 50 परतों पर, यानी 5 मीटर कपड़े की बचत।', ta: 'உதாரணம்: 10 செ.மீ. குறைவு, 50 அடுக்குகளில், 5 மீட்டர் துணி மிச்சம்.', act: 4, teaser: true },
      { say: 'At 150 rupees a metre, that is 750 rupees, on one lay.', en: 'At ₹150 a metre, that is ₹750 saved on one lay.', hi: '₹150 प्रति मीटर पर, एक ले पर ₹750 की बचत।', ta: 'மீட்டருக்கு ₹150 என்றால், ஒரு லேயில் ₹750 மிச்சம்.', act: 5, teaser: true },
      { say: 'CAD software makes this layout for you, and checks it.', hi: 'CAD सॉफ़्टवेयर यह लेआउट आपके लिए बनाता है और जाँचता है।', ta: 'CAD மென்பொருள் இந்த அமைப்பை உங்களுக்காகச் செய்து சரிபார்க்கும்.', act: 6 },
    ],
 },
  {
    id: 'problem', take: 'card', card: 'problem', clip: null,
    beats: [
      { say: 'Many units still mark by hand, with chalk. And commercial CAD is expensive.', hi: 'कई यूनिट आज भी चॉक से हाथ से मार्कर बनाती हैं। और कमर्शियल CAD महँगा है।', ta: 'பல யூனிட்கள் இன்னும் சாக்கால் கையால் மார்க்கர் வரைகின்றன. வணிக CAD விலை அதிகம்.', act: 1 },
    ],
 },
  {
    id: 'marker', take: 'app', section: 'Marker', clip: 'marker',
    beats: [
      { say: 'This is BETA Marker, with a demo shirt.', hi: 'यह BETA Marker है, एक डेमो शर्ट के साथ।', ta: 'இது BETA Marker, ஒரு டெமோ சட்டையுடன்.', act: 'markerDemo' },
      { say: 'Set the fabric width: 1500 millimetres.', en: 'Set the fabric width: 1500 mm.', hi: 'कपड़े की चौड़ाई डालिए: 1500 मिमी।', ta: 'துணியின் அகலம்: 1500 மி.மீ.', act: 'markerWidth' },
      { say: 'Set the size ratio: garments of each size.', hi: 'साइज़ रेशियो डालिए: हर साइज़ के कितने कपड़े।', ta: 'அளவு விகிதம்: ஒவ்வொரு அளவிலும் எத்தனை ஆடைகள்.', act: 'markerRatio' },
      { say: 'Press Nest. BETA places every piece for you.', hi: 'Nest दबाइए। BETA हर पीस खुद रखता है।', ta: 'Nest அழுத்துங்கள். BETA ஒவ்வொரு துண்டையும் தானே வைக்கும்.', act: 'markerNest' },
      { say: 'It tries many layouts, and shows the best so far.', hi: 'यह कई लेआउट आज़माता है और अब तक का सबसे अच्छा दिखाता है।', ta: 'இது பல அமைப்புகளை முயற்சித்து, இதுவரை சிறந்ததைக் காட்டும்.', act: 'markerWait', min: 4, teaser: true },
      { say: 'The result: marker length, and efficiency.', hi: 'नतीजा: मार्कर की लंबाई और एफ़िशिएंसी।', ta: 'முடிவு: மார்க்கர் நீளம், திறன் (efficiency).', act: 'markerResult', teaser: true },
      { say: 'Every marker is checked: no overlaps, and the grain is right.', hi: 'हर मार्कर जाँचा जाता है: कोई ओवरलैप नहीं, ग्रेन सही।', ta: 'ஒவ்வொரு மார்க்கரும் சரிபார்க்கப்படும்: ஒன்றின் மேல் ஒன்று இல்லை, கிரெய்ன் சரி.', act: 'markerChecks' },
      { say: 'Then export a PDF, or a plotter file.', hi: 'फिर PDF या प्लॉटर फ़ाइल निकालिए।', ta: 'பிறகு PDF அல்லது பிளாட்டர் கோப்பு எடுங்கள்.', act: 'markerExport' },
    ],
 },
  {
    id: 'cutplan', take: 'app', section: 'Cut plan', clip: 'cutplan',
    beats: [
      { say: 'Cut plan turns an order into lays.', hi: 'कट प्लान एक ऑर्डर को ले में बदलता है।', ta: 'கட் பிளான் ஒரு ஆர்டரை லேக்களாக மாற்றும்.', act: 'planOpen' },
      { say: 'Enter garments per size, and the highest ply.', hi: 'हर साइज़ के कपड़े और सबसे ऊँची प्लाई डालिए।', ta: 'ஒவ்வொரு அளவுக்கும் ஆடைகள், அதிகபட்ச ப்ளை எண்ணிக்கை.', act: 'planFill' },
      { say: 'BETA works out the lays, the plies, and the metres of fabric.', hi: 'BETA ले, प्लाई और कपड़े के मीटर निकाल देता है।', ta: 'BETA லேக்கள், ப்ளைகள், துணி மீட்டர் கணக்கிடும்.', act: 'planRun', min: 3 },
    ],
 },
  {
    id: 'floor', take: 'app', section: 'Rolls, spreading, bundles', clip: 'cutplan',
    beats: [
      { say: 'Record each fabric roll and check it.', hi: 'हर कपड़े का रोल दर्ज कीजिए और जाँचिए।', ta: 'ஒவ்வொரு துணி ரோலையும் பதிவு செய்து சரிபாருங்கள்.', act: 'rolls' },
      { say: 'Print a spreading sheet for each lay.', hi: 'हर ले के लिए स्प्रेडिंग शीट प्रिंट कीजिए।', ta: 'ஒவ்வொரு லேக்கும் ஸ்ப்ரெடிங் ஷீட் அச்சிடுங்கள்.', act: 'spreading' },
      { say: 'Make bundles, with a QR ticket for every bundle.', hi: 'बंडल बनाइए, हर बंडल का QR टिकट।', ta: 'பண்டல்கள் செய்யுங்கள், ஒவ்வொன்றுக்கும் QR டிக்கெட்.', act: 'bundles' },
    ],
 },
  {
    id: 'quick', take: 'phone', section: 'Photo to marker', clip: 'quick',
    beats: [
      { say: 'No CAD files? Use Quick Marker on your phone.', hi: 'CAD फ़ाइल नहीं है? फ़ोन पर Quick Marker चलाइए।', ta: 'CAD கோப்பு இல்லையா? போனில் Quick Marker பயன்படுத்துங்கள்.', act: 'quickStart', teaser: true },
      { say: 'Photograph the paper pieces, with an A4 sheet for size.', hi: 'काग़ज़ के पीस की फ़ोटो लीजिए, साइज़ के लिए पास में A4 शीट रखिए।', ta: 'காகிதத் துண்டுகளைப் படம் எடுங்கள். அளவுக்காக அருகில் A4 தாள் வையுங்கள்.', act: 'quickPhoto', teaser: true },
      { say: 'BETA finds the outlines. You check and name each piece.', hi: 'BETA आउटलाइन ढूँढता है। आप हर पीस जाँचते और नाम देते हैं।', ta: 'BETA வெளிக்கோடுகளைக் கண்டுபிடிக்கும். நீங்கள் ஒவ்வொன்றையும் சரிபார்த்து பெயரிடுங்கள்.', act: 'quickReview' },
      { say: 'Enter the fabric width and the number of garments.', hi: 'कपड़े की चौड़ाई और कपड़ों की गिनती डालिए।', ta: 'துணி அகலம், ஆடைகளின் எண்ணிக்கை உள்ளிடுங்கள்.', act: 'quickOrder' },
      { say: 'Then BETA makes the marker.', hi: 'फिर BETA मार्कर बनाता है।', ta: 'பிறகு BETA மார்க்கரை உருவாக்கும்.', act: 'quickNest', min: 3, teaser: true },
      { say: 'Print it on A4 sheets, use a chalk guide, or a projector.', hi: 'इसे A4 शीट पर प्रिंट कीजिए, चॉक गाइड लीजिए, या प्रोजेक्टर से दिखाइए।', ta: 'A4 தாள்களில் அச்சிடுங்கள், சாக் வழிகாட்டி, அல்லது ப்ரொஜெக்டரில் காட்டுங்கள்.', act: 'quickUse', teaser: true },
    ],
 },
  {
    id: 'pattern', take: 'app', section: 'Pattern', clip: 'pattern',
    beats: [
      { say: 'BETA Pattern grades a pattern into all sizes.', hi: 'BETA Pattern एक पैटर्न को सारे साइज़ में ग्रेड करता है।', ta: 'BETA Pattern ஒரு பேட்டர்னை எல்லா அளவுகளுக்கும் கிரேடு செய்யும்.', act: 'patternGrade', min: 5 },
      { say: 'Seam checks show if two seams that join have the same length.', hi: 'सीम चेक बताता है कि जुड़ने वाली दो सिलाइयों की लंबाई बराबर है या नहीं।', ta: 'இணையும் இரண்டு தையல்களின் நீளம் சமமா என்று சீம் செக் காட்டும்.', act: 'patternSeams', min: 5 },
      { say: 'Export DXF files for your CAD, or print the pieces full size.', hi: 'अपने CAD के लिए DXF फ़ाइल निकालिए, या पीस पूरे साइज़ में प्रिंट कीजिए।', ta: 'உங்கள் CAD-க்கு DXF கோப்பு எடுங்கள், அல்லது துண்டுகளை முழு அளவில் அச்சிடுங்கள்.', act: 'patternOutput', min: 4 },
    ],
 },
  {
    id: 'proof', take: 'card', card: 'proof', section: 'Honest numbers', clip: null,
    beats: [
      { say: 'Is it good? We test it in public.', hi: 'क्या यह अच्छा है? हम इसे सबके सामने जाँचते हैं।', ta: 'இது நல்லதா? வெளிப்படையாகச் சோதிக்கிறோம்.', act: 1 },
      { say: "On 300 tasks from Lectra's open data, at 60 seconds each, BETA was 0.73 points below Lectra on average, and 1.22 at the median.", en: '300 Lectra tasks, 60 s each: BETA was 0.73 points below Lectra on average, 1.22 below at the median.', hi: 'Lectra के पब्लिक डेटा के 300 काम, हर एक 60 सेकंड: औसत में BETA, Lectra से 0.73 पॉइंट नीचे; मीडियन 1.22 नीचे।', ta: 'Lectra பொது தரவின் 300 பணிகள், ஒவ்வொன்றும் 60 விநாடி: சராசரியில் BETA 0.73 புள்ளி குறைவு; மீடியன் 1.22 குறைவு.', act: 2 },
      { say: '84 wins, 12 ties, 204 losses. Close, but not better yet.', hi: '84 जीत, 12 बराबर, 204 हार। क़रीब है, पर अभी बेहतर नहीं।', ta: '84 வெற்றி, 12 சமம், 204 தோல்வி. நெருக்கம், ஆனால் இன்னும் சிறந்தது இல்லை.', act: 3 },
      { say: 'And every marker is checked before export.', hi: 'और हर मार्कर एक्सपोर्ट से पहले जाँचा जाता है।', ta: 'ஒவ்வொரு மார்க்கரும் ஏற்றுமதிக்கு முன் சரிபார்க்கப்படும்.', act: 4 },
    ],
 },
  {
    id: 'feedback', take: 'app', clip: null,
    beats: [
      { say: 'Found a problem? Press Feedback, and tell us.', hi: 'कोई दिक्कत मिली? Feedback दबाइए और हमें बताइए।', ta: 'பிரச்சனை கண்டீர்களா? Feedback அழுத்தி சொல்லுங்கள்.', act: 'feedback' },
    ],
 },
  {
    id: 'close', take: 'card', card: 'close', clip: null, vertical: true,
    beats: [
      { say: 'BETA is free, works offline, and your data stays in your factory.', hi: 'BETA मुफ़्त है, बिना इंटरनेट चलता है, और आपका डेटा आपकी फ़ैक्टरी में रहता है।', ta: 'BETA இலவசம், இணையம் இல்லாமல் இயங்கும், உங்கள் தரவு உங்கள் தொழிற்சாலையிலேயே.', act: 1 },
      { say: 'It is a young project, made for fun with modern AI tools. We need your help.', hi: 'यह एक नया प्रोजेक्ट है, आधुनिक AI टूल्स की मदद से मज़े के लिए बना। हमें आपकी मदद चाहिए।', ta: 'இது ஒரு புதிய திட்டம், நவீன AI கருவிகளுடன் ஆர்வத்துக்காக உருவானது. உங்கள் உதவி தேவை.', act: 2 },
      { say: 'Try it now in your browser, on our website.', en: 'Try it now in your browser: sohitqwer-del.github.io/beta-cutting-room', hi: 'अभी अपने ब्राउज़र में आज़माइए: sohitqwer-del.github.io/beta-cutting-room', ta: 'இப்போதே உங்கள் பிரவுசரில் முயற்சியுங்கள்: sohitqwer-del.github.io/beta-cutting-room', act: 3, min: 5, teaser: true },
    ],
 },
];

/** Flat list of beats with stable ids, e.g. "marker-3". */
export function allBeats() {
  return scenes.flatMap((s) => s.beats.map((b, i) => ({ ...b, id: `${s.id}-${i + 1}`, scene: s.id, index: i, en: b.en ?? b.say })));
}

/** Feature clips for the Help page: name -> scene ids. */
export function clips() {
  const out = {};
  for (const s of scenes) if (s.clip) (out[s.clip] ??= []).push(s.id);
  return out;
}
