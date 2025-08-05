# 🔍 Speedy - Your Instant Windows Search Companion

> *"Because Windows search should be fast, not frustrating."*

---

## 🚀 What is Speedy?

**Speedy** is a lightning-fast, Spotlight-inspired search tool for Windows.  
After switching from macOS, I was tired of:

- 🐌 **Slow searches** (why does File Explorer take 10 seconds?)
- 🤔 **Weird results** (why is Bing showing up when I search for my own files?)
- 😤 **Clunky UI** (why does it feel like I'm using Windows XP?)

So I built Speedy:  
Just **press Spacebar**, type, and get **instant results** — no waiting, no nonsense.

---

## ✨ Features

### ⚡ Blazing Fast  
- Real-time search through files, folders, and apps  
- Smart ranking for frequently used items  

### 🎨 Beautiful & Minimal  
- Clean, macOS Spotlight-inspired UI  
- Smooth animations + transparent blur effects  
- Auto dark/light mode support  

### ⌨️ Keyboard First  
- `Spacebar` → Open search  
- `↑ / ↓` → Navigate results  
- `Enter` → Open selected item  
- `Esc` → Close  

### 🤖 No Bloat  
- ❌ No telemetry (your searches stay private)  
- ❌ No ads (unlike Windows search...)  
- 🧠 Lightweight (~50MB RAM)  

---

## 🛠️ Tech Stack

| Frontend              | Backend                   |
|-----------------------|----------------------------|
| React + Vite          | Rust (via Tauri)           |
| CSS Animations        | SQLite Database            |
| Framer Motion *(TBD)* | Windows Shell Integration  |

---

## 📦 Coming Soon

- [ ] Custom themes  
- [ ] Plugin support  
- [ ] File preview pane  

---

## 💡 Why "Speedy"?

Because **searching should feel instant**—not like loading a webpage from 2006.

---

## 🚀 Quick Start

1. **Clone & Enter**
   ```sh
   git clone https://github.com/SinofPride-999/Speedy.git
   cd speedy

2. **Install Dependencies**
    npm install  # Frontend
    cargo install tauri-cli  # Rust (if needed)

3. **Run Dev Mode**
    npm run tauri dev

---
