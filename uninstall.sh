cat > uninstall.sh << 'EOF'
#!/bin/bash
# Uninstall vortex CLI

echo "🗑️  Uninstalling vortex..."

if [ -f "/usr/local/bin/vortex" ]; then
    sudo rm /usr/local/bin/vortex
    echo "✅ Uninstalled successfully"
else
    echo "⚠️  Vortex not found at /usr/local/bin/vortex"
fi

# Also clean cgroups if any exist
if [ -d "/sys/fs/cgroup/vortex" ]; then
    echo "🧹 Cleaning up vortex cgroups..."
    sudo find /sys/fs/cgroup/vortex -type d -delete 2>/dev/null || true
    echo "✅ Cgroups cleaned"
fi

echo "✅ Done!"
EOF

chmod +x uninstall.sh