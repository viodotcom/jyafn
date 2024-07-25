set -e

gh release download -p "*.whl" -D /tmp/jyafn-wheels
twine upload /tmp/jyafn-wheels/*.whl
rm -rf /tmp/jyafn-wheels
