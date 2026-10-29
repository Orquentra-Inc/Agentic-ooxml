import sharp from 'sharp'
const [,, file, h = '1400'] = process.argv
const m = await sharp(file).metadata()
const H = Number(h); let i = 0
for (let y = 0; y < m.height; y += H, i++) {
  await sharp(file).extract({ left: 0, top: y, width: m.width, height: Math.min(H, m.height - y) }).toFile(file.replace('.png', `-s${i}.png`))
}
console.log(i, 'slices')
