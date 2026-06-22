# MiOS-Mobile-Beta
Это Исходный код самой первой версии MiOS Mobile! Оцените!

вот все зависимости для компиляции MiOS Mobile Beta:
Основные пакеты:
Ubuntu/Debian:
bash
sudo apt update
sudo apt install -y \
    nasm \
    grub-pc-bin \
    grub-common \
    xorriso \
    qemu-system-x86 \
    build-essential \
    curl \
    git
    
Fedora/RHEL:
bash
sudo dnf install -y \
    nasm \
    grub2-tools \
    grub2-pc-modules \
    xorriso \
    qemu-system-x86 \
    gcc \
    make \
    curl \
    git
    
Arch Linux:
bash
sudo pacman -S \
    nasm \
    grub \
    libisoburn \
    qemu-system-x86 \
    base-devel \
    curl \
    git
    
Rust и компоненты:
bash
# Установка Rust (если ещё не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Настройка проекта
make setup
