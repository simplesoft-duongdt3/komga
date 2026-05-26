FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY config.py db.py walker.py diff.py api.py applier.py main.py server.py ./

EXPOSE 5050
CMD ["python", "server.py"]
