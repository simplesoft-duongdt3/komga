FROM python:3.12-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY config.py db.py walker.py diff.py api.py applier.py main.py server.py export_json.py ./

EXPOSE 5050
CMD ["python", "server.py"]
