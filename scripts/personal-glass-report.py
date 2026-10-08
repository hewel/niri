#!/usr/bin/env python3
"""Summarize the opt-in shader probe and create a local image comparison slider."""
import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    directory = args.directory.resolve()
    record = json.loads((directory / "comparison.json").read_text())
    x, y, width, height = record["geometry"]
    box = (x, y, x + width, y + height)
    interior = (x + 48, y + 48, x + width - 48, y + height - 48)

    def measure(first, second, crop):
        with Image.open(directory / f"{first}.png") as a, Image.open(directory / f"{second}.png") as b:
            diff = ImageChops.difference(a.convert("RGB").crop(crop), b.convert("RGB").crop(crop))
        return {"pixels": diff.width * diff.height,
                "changed_pixels": sum(pixel != (0, 0, 0) for pixel in diff.get_flattened_data()),
                "mean_abs_rgb_255": sum(ImageStat.Stat(diff).mean) / 3,
                "max_channel_delta": max(high for _, high in diff.getextrema())}

    measurements = {
        "old_vs_aligned": measure("old-preset", "upstream-preset", box),
        "aligned_vs_reference": measure("upstream-preset", "upstream-reference", box),
        "aligned_vs_reference_interior": measure("upstream-preset", "upstream-reference", interior),
        "blur_vs_sharp": measure("upstream-preset", "upstream-preset-sharp", box),
        "reference_shader_sha256": hashlib.sha256((directory / "reference.frag").read_bytes()).hexdigest(),
        "ported_shader_sha256": hashlib.sha256((directory / "ported.frag").read_bytes()).hexdigest(),
        "source_commit": record["source_commit"],
        "tracked_changes": record["tracked_changes"],
        "renderer_request": record["renderer_request"],
    }
    assert measurements["blur_vs_sharp"]["changed_pixels"] > 0, "Probe did not produce observable blur"
    (directory / "measurements.json").write_text(json.dumps(measurements, indent=2) + "\n")
    html = """<!doctype html><html lang="zh-CN"><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Liquid Glass SDR 对比</title>
<style>body{font:16px system-ui,sans-serif;background:#14202e;color:#edf4f8;margin:24px auto;padding:0 20px;max-width:1100px}button,select,input{font:inherit}select{padding:8px;max-width:100%}.stage{position:relative;aspect-ratio:3/2;margin-top:18px}.stage img{position:absolute;width:100%;height:100%;inset:0}.stage:after{content:'';position:absolute;inset:0 auto 0 var(--split,50%);width:2px;background:white;pointer-events:none}input[type=range]{width:100%}.labels{display:flex;justify-content:space-between;gap:12px}p{line-height:1.6}pre{overflow:auto;padding:16px;background:#0c1622;font-size:13px}</style>
<h1>Liquid Glass：相同 SDR 场景对比</h1>
<p>固定 960×640 纹理、640×544 窗口、位置 (80,48)、不透明 sRGB 背景。原版参考来自固定提交的未经修改着色器；这不是原版完整合成器或真实 HDR 验证。图中隔离了背景效果，没有 GTK 窗口的底色和透明度。</p>
<label for="pair">比较：</label><select id="pair">
<option value="presets">旧测试预设 / 对齐原版预设</option>
<option value="reference" selected>移植版 / 原版着色器（相同原版预设、相同模糊）</option>
<option value="sharp">移植版 / 原版着色器（关闭底层模糊）</option>
<option value="blur">移植版：开启 / 关闭底层模糊</option>
<option value="ordinary">普通模糊 / 原版预设 Glass</option></select>
<div class="labels"><p id="left-label"></p><p id="right-label"></p></div>
<div class="stage" id="stage"><img id="right" alt="右侧对照"><img id="left" alt="左侧对照"></div>
<label for="split">拖动分界线：</label><input id="split" type="range" min="0" max="100" value="50">
<p>中央区域排除 48 像素边缘带后，量化的是 RGB 误差；这些数值不能代替自然墙纸和真实窗口的视觉验收。</p>
<details><summary>测量与来源</summary><pre id="record"></pre></details>
<script>const pairs={presets:['old-preset','upstream-preset','旧测试预设','对齐原版预设'],reference:['upstream-preset','upstream-reference','移植版','原版着色器'],sharp:['upstream-preset-sharp','upstream-reference-sharp','移植版：无底层模糊','原版：无底层模糊'],blur:['upstream-preset','upstream-preset-sharp','移植版：有底层模糊','移植版：无底层模糊'],ordinary:['ordinary-blur','upstream-preset','普通模糊','原版预设 Glass']};
const select=document.getElementById('pair'),slider=document.getElementById('split');
function update(){const p=pairs[select.value];document.getElementById('left').src=p[0]+'.png';document.getElementById('right').src=p[1]+'.png';document.getElementById('left-label').textContent=p[2];document.getElementById('right-label').textContent=p[3];}
function split(){document.getElementById('left').style.clipPath='inset(0 '+(100-slider.value)+'% 0 0)';document.getElementById('stage').style.setProperty('--split',slider.value+'%');}
select.addEventListener('change',update);slider.addEventListener('input',split);update();split();
document.getElementById('record').textContent=JSON.stringify(RECORD,null,2);</script></html>"""
    payload = json.dumps({"conditions": record, "measurements": measurements}, ensure_ascii=False).replace("<", "\\u003c")
    (directory / "comparison.html").write_text(html.replace("JSON.stringify(RECORD,", "JSON.stringify(" + payload + ","))
    print(json.dumps(measurements, indent=2))
    print(directory / "comparison.html")


if __name__ == "__main__":
    main()
