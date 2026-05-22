#!/bin/bash

# Ensure the script is run with sudo
if [ "$EUID" -ne 0 ]; then
  echo "Ce script doit être exécuté en tant que root. Utilisez 'sudo ./fix_tablet_driver.sh'"
  exit 1
fi

echo "=== Résolution des dépendances système (System Tray) ==="
# Installation de la librairie manquante pour Arch Linux
if command -v pacman &> /dev/null; then
    echo "Arch Linux détecté. Installation de libayatana-appindicator..."
    pacman -S --needed --noconfirm libayatana-appindicator
elif command -v apt-get &> /dev/null; then
    echo "Debian/Ubuntu détecté. Installation de libayatana-appindicator3-1..."
    apt-get update
    apt-get install -y libayatana-appindicator3-1
elif command -v dnf &> /dev/null; then
    echo "Fedora détecté. Installation de libappindicator-gtk3..."
    dnf install -y libappindicator-gtk3
else
    echo "Gestionnaire de paquets non reconnu. Veuillez installer libayatana-appindicator3 ou libappindicator3 manuellement."
fi

echo -e "\n=== Configuration de uinput (Virtual Devices) ==="
# 1. Charger le module noyau uinput
echo "Chargement du module noyau 'uinput'..."
modprobe uinput
# Le rendre persistant au redémarrage
echo "uinput" > /etc/modules-load.d/uinput.conf

# 2. Créer les règles udev
echo "Création de la règle udev pour /dev/uinput..."
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' > /etc/udev/rules.d/99-uinput.rules

# 3. Ajouter l'utilisateur au groupe 'input'
# Récupérer l'utilisateur d'origine qui a lancé la commande sudo
TARGET_USER=${SUDO_USER:-$(logname)}
if [ -n "$TARGET_USER" ]; then
    echo "Ajout de l'utilisateur $TARGET_USER au groupe 'input'..."
    usermod -aG input "$TARGET_USER"
else
    echo "Impossible de déterminer l'utilisateur. Veuillez l'ajouter manuellement : sudo usermod -aG input \$USER"
fi

# 4. Recharger les règles udev
echo "Rechargement des règles udev..."
udevadm control --reload-rules
udevadm trigger

echo -e "\n=== Terminé ==="
echo "IMPORTANT : Pour que l'ajout au groupe 'input' soit pris en compte, vous devez vous DÉCONNECTER puis vous RECONNECTER (ou redémarrer votre ordinateur)."
echo "Ensuite, relancez 'cargo run'."
